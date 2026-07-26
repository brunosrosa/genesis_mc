# PRD-10.1: Specification for Local Model Manager & Cognitive Repair Daemon

**Status:** Especificação Proposta (Aguardando Aprovação HITL)  
**Módulo:** Daemon de Gerenciamento de IA Local (`soda-model-manager`)  
**Target Hardware:** NVIDIA RTX 2060 Mobile (Teto Rígido de 6.0 GB VRAM, GDDR6) + Intel i9 (32GB RAM Host)  
**Arquitetura:** Rust (Tokio Async Runtime, Zero-Copy IPC)  
**Rastreabilidade ADR:** ADR-027, ADR-029, ADR-032, ADR-033, ADR-034, ADR-035, ADR-036  

---

## 1. Objetivo Atômico

O **PRD-10.1** especifica os requisitos de engenharia para o desenvolvimento do daemon **`soda-model-manager`** em Rust (Tokio). O objetivo principal é orquestrar autonomamente o ciclo de vida dos modelos locais em formato GGUF, gerenciar o hot-swapping de Small Language Models (SLMs de 1.5B a 4B) e garantir a estabilidade termodinâmica da VRAM na GPU NVIDIA RTX 2060m, respeitando o limite físico inflexível de **6.0 GB de VRAM**.

O daemon atua como o Guardião da VRAM e Roteador de Inferência do SODA V5, impedindo que requisições ou modelos extrapolem o orçamento de memória e provoquem o estresse no barramento PCIe Gen3 por paginação no Host (*spillover*).

---

## 2. Sincronizador de Frota e Auto-Profiling $\mathcal{O}(1)$

Para manter o ecossistema agêntico atualizado com os melhores SLMs sem desperdiçar largura de banda de rede nem capacidade de disco NVMe, o daemon implementará um serviço de sincronia de frota baseado em **Auto-Profiling em Tempo Constante $\mathcal{O}(1)$**.

```
[ Sincronizador de Frota (Background Task Tokio) ]
                       │
                       ▼
    (Consulta Header Remoto via hf-hub Crate)
                       │
                       ▼
       [ Parsing GGUF Metadata Header O(1) ]
       - Contagem de Camadas & Parâmetros
       - Tipo de Quantização (Q4_K_M, IQ3_M, Ternária)
       - Tamanho Estimado de Pesos (VRAM_Estática)
                       │
                       ▼
     [ Equação de Projeção de VRAM SODA V5 ]
    VRAM_Proj = VRAM_Estática + VRAM_KV_Cache(Ctx_Max) + VRAM_Buffer_CUDA
                       │
         ┌─────────────┴─────────────┐
         ▼                           ▼
(VRAM_Proj > 5.0 GB)       (VRAM_Proj <= 5.0 GB e Desempenho Superior)
         │                           │
         ▼                           ▼
[ Rejeição Automática ]    [ Notificação Agent Inbox (HITL) ]
(Zero Download)            - Alerta de Upgrade Sugerido
                           - Aguarda Autorização Humana
```

### 2.1 Inspeção de Metadados em Cabeçalho Remoto (Zero-Download)
* O daemon consultará repositórios remotos (HuggingFace Hub) utilizando a crate nativa em Rust **`hf-hub`**.
* É terminantemente **PROIBIDO** realizar o download de arquivos `.gguf` massivos (gigabytes) para fins de teste ou avaliação. O daemon fará requisições HTTP de intervalo parcial (*Range Requests*) para ler apenas os bytes do **cabeçalho de metadados GGUF** (primeiros kilobytes do arquivo).

### 2.2 Algoritmo de Auto-Profiling $\mathcal{O}(1)$
Ao extrair a estrutura de tensores, contagem de camadas, dimensões de atenção e precisão de quantização do cabeçalho remoto, o daemon calculará a estimativa de consumo de VRAM usando a equação de projeção SODA:

$$\text{VRAM}_{\text{Projetada}} = \text{VRAM}_{\text{Estática (Pesos)}} + \text{VRAM}_{\text{KV\_Cache}}(\text{Ctx}_{\text{Alvo}}) + \text{VRAM}_{\text{Buffer\_CUDA}} \text{ (MB)}$$

* **Critério de Rejeição Implicável:** Se $\text{VRAM}_{\text{Projetada}} > 5.000 \text{ MB}$ ($5,0 \text{ GB}$), o modelo é classificado instantaneamente como incompatível e ignorado sem efetuar download.

### 2.3 Integração com Agent Inbox (Human-In-The-Loop)
* Se o Auto-Profiling indicar que um modelo remoto candidato possui acurácia superior aos modelos atuais e cabe confortavelmente dentro do orçamento de 5.0 GB de VRAM, o daemon **não realizará o download de forma autônoma**.
* O daemon enviará um evento estruturado para a **Agent Inbox** do usuário contendo o relatório de comparativo FinOps. A ação de download e substituição do modelo exigirá a confirmação manual e explícita do Arquiteto Humano (HITL).

---

## 3. Reparo Cognitivo & Incorporação de ADRs (V5 Canon)

O daemon `soda-model-manager` embarcará nativamente no pipeline de inferência as Leis Duras de Reparo Cognitivo formalizadas nos ADRs recentes:

### 3.1 ADR-034: Modulação de Ativação via Vetores RepE (`cb_eval`)
* O motor de inferência em Rust (`llama-cpp-2` / `mistral.rs`) registrará interceptadores na função de callback de avaliação de tensores (`cb_eval`).
* Durante a passagem direta (*forward pass*), o daemon aplicará vetores de controle RepE em formato GGUF nas **camadas intermediárias (faixa de 10 a 24)** com fator de escala $\alpha \in [0.15, 0.60]$, corrigindo distorções provocadas pela quantização agressiva sem inflar o consumo de VRAM ($< 1\text{ MB}$ de footprint).

### 3.2 ADR-035: Reparo Sintático Zero-Token via IPC (`jsonrepair`)
* Todo o fluxo de saída em streaming do SLM será interceptado na camada de transporte IPC do Gateway Rust.
* Se a verificação sintática via `serde_json` falhar, o buffer será submetido síncronamente à cura por crates nativas em Rust (`jsonrepair` ou `llm_json`).
* O reparo executará stripping de cercas Markdown, autocompletude por pilha de delimitadores para JSONs truncados e coerção de literais em **$< 1,0 \text{ ms}$** com **Custo Zero de VRAM e Tokens**, eliminando re-prefills na GPU.

### 3.3 ADR-036: VRAM Assimétrica e Ralph Loop Sequencial
* Fica banida qualquer forma de amostragem paralela (*Best-of-N*) na GPU para evitar a multiplicação de KV Caches na VRAM.
* O sistema utilizará o **Ralph Loop Sequencial**: o modelo primário (3B-4B) opera na GPU VRAM, enquanto o modelo verificador (*Critic*, ex: BitNet 1.58-bit ou GLiClass) opera $100\%$ desacoplado na RAM do Host via CPU (AVX2/AVX-512).
* Em caso de rejeição pelo *Critic*, o re-prompt utilizará compulsoriamente **KV Cache Prefix Reuse**, reaproveitando os tensores de atenção já calculados no prompt base e restringindo o processamento aos tokens de correção.

### 3.4 Sanitização Termodinâmica do KV Cache (Implementação Estrita do ADR-027)
* O motor `llama_engine.rs` DEVE obrigatoriamente ser refatorado para usar **Cache KV Assimétrico**.
* O contexto deve ser instanciado compulsoriamente com `with_type_k(KvCacheType::F16)` (para preservar a rotação RoPE livre de distorções numéricas) e `with_type_v(KvCacheType::Q4_K)` (para esmagar o footprint de 30k tokens para $< 1 \text{ GB}$ na VRAM).

---

## 4. Definition of Done (DoD)

Para que a futura fase de implementação via TDD (Red-Green-Refactor) seja concluída com sucesso, o repositório exigirá a aprovação dos seguintes critérios automatizados no compilador e na suíte de testes Rust:

### 4.1 Rejeição Prematura de Sobrecarga de VRAM (Testes Unitários)
* **Teste:** `test_auto_profiling_rejects_overbudget_gguf()`
* **DoD:** Provar que a leitura isolada de metadados GGUF de modelos que demandem $\ge 5,1 \text{ GB}$ de VRAM projeta a rejeição em tempo $\mathcal{O}(1)$ sem realizar download de chunks de dados adicionais.

### 4.2 Reparo Sintático Sub-Milissegundo (Testes de Performance)
* **Teste:** `test_response_healing_sub_millisecond_json_repair()`
* **DoD:** Demonstrar que buffers de JSON truncados ou corrompidos contendo cercas Markdown e chaves abertas são convertidos em estruturas RFC 8259 válidas pelo `jsonrepair` em tempo estritamente inferior a **$1,0 \text{ ms}$** ($< 1000 \mu \text{s}$).

### 4.3 Isolamento da VRAM & Prefix Reuse (Testes de Integração)
* **Teste:** `test_ralph_loop_prefix_reuse_zero_vram_leak()`
* **DoD:** Validar que o loop de reflexão sequencial reaproveita os ponteiros da KV Cache do modelo primário na GPU sem alocar buffers paralelos e mantendo o *Critic* $100\%$ isolado na CPU Host.

### 4.4 Inicialização de Contexto com KV Cache Assimétrico (Testes Unitários)
* **Teste:** `test_llama_engine_context_init_asymmetric_kv_cache()`
* **DoD:** Deve existir um teste de inicialização do contexto provando que a tipagem do KV Cache foi alocada nativamente como F16 para Keys (`KvCacheType::F16`) e Q4_K para Valores (`KvCacheType::Q4_K`), esmagando o footprint da janela estendida para $< 1 \text{ GB}$ de VRAM.

### 4.5 Higiene de Código & Compilação Limpa
* **DoD:** O projeto DEVE atingir **Exit Code 0** na execução do comando de checagem do compilador sem nenhum aviso de linter ou supressão indevida:
  ```bash
  cargo check --all-targets -D warnings
  ```

---

## 5. Conclusão & Alinhamento HITL

Este artefato de especificação consolida a arquitetura técnica do gerenciador de IA local do SODA V5. Nenhuma mutação de código-fonte foi realizada nesta etapa.

**As correções do Cache KV Assimétrico foram injetadas. Aguardo o sinal verde final para iniciarmos a fase de TDD (Red-Green-Refactor) em Rust.**
