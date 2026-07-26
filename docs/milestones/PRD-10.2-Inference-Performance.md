# PRD-10.2: Specification for Inference Performance & Thermal CPU Affinity Isolation

**Status:** Especificação Proposta (Aguardando Aprovação HITL)  
**Módulo:** Motor de Inferência & Governor Térmico (`llama_engine.rs` / `soda_thermal_governor.rs`)  
**Target Hardware:** NVIDIA RTX 2060 Mobile (6.0 GB VRAM) + Intel i9 (Threads/Cores Físicos Dedicados)  
**Arquitetura:** Rust (Tokio Async Runtime, `core_affinity`, Especulação `ngram-mod`)  
**Rastreabilidade ADR:** ADR-027, ADR-032, ADR-033, ADR-036  

---

## 1. Objetivo Atômico

O **PRD-10.2** especifica os requisitos de engenharia de alta performance para otimizar a velocidade de geração autorregressiva ($S \ge 1,15\times \text{ a } 1,45\times$) e garantir a estabilidade térmica do SO. O objetivo é implementar no motor `llama_engine.rs`:

1. **Aceleração N-Gram Zero-VRAM:** Integração do algoritmo determinístico `--spec-type ngram-mod` na RAM do Host, acelerando a geração de JSONs e estruturas AST sem consumir memória de vídeo.
2. **Isolamento Térmico de CPU via Afinidade de Núcleo (`core_affinity`):** Mapeamento e fixação física das threads do *Critic Model* (ou workers de micro-sidecars de CPU) em núcleos dedicados do processador Intel i9, prevenindo a disputa por recursos com a piscina de threads do Tokio e estancando o estrangulamento térmico do sistema operativo.

---

## 2. Incorporação de ADRs de Performance & Termodinâmica

### 2.1 ADR-032: Adoção N-Gram Zero-VRAM na RAM Host
* **Aceleração Determinística:** O motor `llama_engine.rs` DEVE ativar a especulação baseada em n-gramas (`ngram-mod`) para saídas estruturadas (JSON Schemas, ASTs, SQL).
* **Zero VRAM Footprint:** A tabela de hash de n-gramas deve ser alocada estritamente na memória RAM central do sistema ($\sim 16 \text{ MB}$ de footprint), garantindo **$0 \text{ MB}$ de consumo adicional na VRAM** da dGPU RTX 2060m.
* **Mapeamento de Parâmetros:** A janela de amostragem n-gram usará parâmetros calibrados: `n_match = 24`, `n_min = 48`, `n_max = 64`.

### 2.2 ADR-033: Isolamento Físico via Afinidade de Núcleo (`core_affinity`)
* **Thread Pinning em Rust:** O daemon usará a crate **`core_affinity`** para ancorar fisicamente as threads do *Critic Model* (executando em CPU via AVX2) em um conjunto de núcleos isolados (ex: Cores 0 a 3).
* **Proteção do Event Loop Tokio:** A piscina de threads do Tokio (`tokio::runtime::Builder`) será configurada para utilizar os núcleos restantes (ex: Cores 4 a $N$), impedindo *context switching* desordenado e surtos de latência no *Time-To-First-Token* (TTFT).
* **Estabilidade Térmica:** O isolamento evita que picos de uso da CPU afetem o *stream* CUDA da dGPU, mantendo a temperatura da máquina abaixo do teto de estrangulamento de $82^\circ\text{C}$.

---

## 3. Definition of Done (DoD)

Para que a futura fase de implementação TDD (Red-Green-Refactor) do PRD-10.2 seja concluída, o repositório exigirá a aprovação dos seguintes testes unitários e de integração em Rust:

### 3.1 Alocação do Buffer N-Gram Zero-VRAM (Testes Unitários)
* **Teste:** `test_ngram_specation_buffer_allocation_host_ram()`
* **DoD:** Provar que a inicialização do buffer de especulação N-Gram aloca a tabela de hash na RAM do Host (pegada $< 20\text{ MB}$) sem instanciar tensores de rascunho adicionais na VRAM da dGPU.

### 3.2 Fixação de Afinidade de CPU via `core_affinity` (Testes de Integração)
* **Teste:** `test_critic_worker_core_affinity_pinning()`
* **DoD:** Demonstrar que o worker do *Critic Model* (ou sidecar de CPU) executa com sucesso o comando de ancoragem `core_affinity::set_for_current()` e retorna os IDs dos núcleos físicos atribuídos sem invadir os núcleos do Tokio.

### 3.3 Higiene & Compilação Limpa
* **DoD:** Compilação com **Exit Code 0** sem nenhum aviso de linter ou erro de compilação:
  ```bash
  cargo check --all-targets --features llama_backend -D warnings
  ```

---

## 4. Conclusão & Alinhamento HITL

Este artefato de especificação consolida a arquitetura de aceleração N-Gram e isolamento de CPU do SODA V5. Nenhum código de execução do PRD-10.2 foi escrito nesta etapa.
