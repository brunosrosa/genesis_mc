# ESPECIFICAÇÃO DE ENGENHARIA BARE-METAL: MARCO 5.4.0
## 🚀 GIGATOKEN AUTO-CURATIVO, PREFILL BYPASS E INFERÊNCIA DA dGPU NO SODA V4

### 🏛️ 1. O Racional de Design (A Biela de Alta Vazão)
O hot path de inferência na dGPU RTX 2060m (6GB) para o **Qwen 3.5 Coder 4B** (Tier 1) exige latência de resposta inicial imediata ($TTFT \le 150\text{ms}$). A abordagem convencional de trafegar texto bruto via IPC, serializar em JSON, trafegar pela ponte FFI para o C++, deserializar e re-tokenizar no backend do `llama.cpp` é um ralo de clock de CPU e largura de banda do barramento PCIe Gen3 x8.

O **Marco 5.4.0** introduz o **Gigatoken**: um mecanismo de tokenização em tempo de execução ($\mathcal{O}(1)$) que roda a **24 GB/s na CPU** utilizando instruções SIMD/AVX2 nativas, convertendo a string de contexto diretamente em um vetor binário de IDs de tokens (`Vec<u32>`). Este vetor binário é injetado diretamente no lote FFI (`LlamaBatch`), realizando o bypass absoluto de re-parsing e re-tokenização na GPU, reduzindo o tráfego do barramento PCIe em até **71%**.

---

### ⚙️ 2. Arquitetura do Componente Auto-Curativo (`GigaTokenEncoder`)

Para garantir o **Agnosticismo de Hardware (ADR-027)** e a **Resiliência Offline**, o `GigaTokenEncoder` implementa o **Caminho 3 de Autocura de Vocabulário**:

1. **Bootstrapping**: No boot do SODA, o `GigaTokenEncoder` (inicializado via `std::sync::OnceLock`) verifica se o arquivo `tokenizer.json` correspondente ao modelo gerador Qwen existe no disco do host.
2. **Extração Física do GGUF (Self-Healing)**: Se o manifesto `.json` estiver ausente, o motor aciona o extrator local. Ele lê o cabeçalho binário do arquivo `.gguf` real via `mmap2` (através da FFI do `llama.cpp`) e extrai as strings brutas de todos os tokens de $0$ até o limite retornado por `llama_n_vocab`.
3. **Compilação JIT de Vocabulário**: O extrator compila dinamicamente esses tokens em uma estrutura BPE válida e grava o arquivo `tokenizer_recovered.json` no SSD do host, inicializando o `tokenizers::Tokenizer` nativo em Rust a partir do arquivo recuperado de forma 100% transparente.

```
                  [Verificação de Boot]
                            │
              ┌─────────────┴─────────────┐
     (tokenizer.json existe?)     (tokenizer.json ausente)
              │                           │
              ▼                           ▼
     [Carrega do Disco]           [Lê Cabeçalho GGUF]
              │                           │
              │                           ▼
              │                  [Extrai Vocab FFI]
              │                           │
              │                           ▼
              │                  [Grava tokenizer_recovered.json]
              │                           │
              └─────────────┬─────────────┘
                            │
                            ▼
               [GigaTokenEncoder ATIVO]
```

---

### 💾 3. Modelagem de Adaptadores e Interfaces (Bypass de Prefill)

#### 3.1. Adaptador de Entrada (`inference_adapter.rs`)
Definição da enum flexível e da injeção opcional de payloads pré-tokenizados:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InferenceInput {
    RawText(String),
    PreTokenized(Vec<u32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulsInferenceRequest {
    pub request_id: String,
    pub input: Option<InferenceInput>,
    pub max_tokens: usize,
    pub temperature: f32,
    pub min_p: f32,
}
```

#### 3.2. FFI Safety & Alinhamento de Tipos (`llama_engine.rs`)
O motor principal do `LlamaCppEngine` intercepta a entrada pré-tokenizada. É mandatória a conversão segura e alinhada dos IDs de `u32` para `i32` (`LlamaToken`) para evitar estouros de sinal ou panics de FFI na ponte C++:

```rust
// No hot path de prefill em llama_engine.rs:
match input {
    InferenceInput::PreTokenized(token_ids) => {
        // Ignora a chamada síncrona de model.str_to_token (llama_tokenize)
        // Executa a conversão de tipo e o preenchimento do lote
        batch.clear(); // Saneamento compulsório contra Logit Leak
        
        for (i, &id) in token_ids.iter().enumerate() {
            let LlamaToken = id as i32; // Alinhamento FFI u32 -> i32
            let is_last = i == token_ids.len() - 1;
            
            // Insere o token no lote FFI
            batch.add(LlamaToken, i as i32, &[0], is_last);
            
            // Isola os logits estritamente no último token
            if is_last {
                batch.set_logits(i as i32, true);
            }
        }
        
        // Dispara o decode de prefill direto na GPU
        model.decode(&mut batch)?;
    }
    InferenceInput::RawText(text) => {
        // Caminho legado sínclono de tokenização na GPU
        let token_ids = model.tokenize(&text)?;
        // ... (lógica padrão de prefill)
    }
}
```

---

### 🧮 4. Governança Termodinâmica de VRAM (RTX 2060m - 6GB)

O limitador de memória de vídeo de 6GB é imposto por uma equação matemática estrita executada no teste de TDD `test_vram_budget_math`:

$$\text{VRAM}_{\text{total}} = \text{VRAM}_{\text{pesos\_Qwen}} + \text{VRAM}_{\text{KV\_Cache}} \le 5632 \text{ MB} \quad (5.5 \text{ GB})$$

* **Pesos do Qwen 3.5 Coder 4B (Q4_K_M)**: $\approx 2.50\text{ GB}$ (2560 MB) alocados estritamente via `n_gpu_layers = 99` na dGPU.
* **KV Cache Quantizado (Q4_K)**: Esmaga o buffer de contexto longo de 32k tokens de $3.66\text{ GB}$ (FP16) para apenas **$~937.5\text{ MB}$** na VRAM.
* **Margem de Respiro do Sistema (Tauri/Host Windows DWM)**: $\ge 500\text{ MB}$ (512 MB) livres na placa, impedindo travamento de renderização e congelamento de tela.

---

### 🔬 5. Caderno de TDD: Contratos de Testes de Integração

A suíte de testes de integração deve ser executada serialmente sob o `TELEMETRY_TDD_LOCK` e provar os seguintes contratos:

1. **`test_gigatoken_prefill_bypass`**:
   - *Ação*: Tokeniza o prompt de entrada `"Refatore o código do SODA"` com o `GigaTokenEncoder` CPU gerando `Vec<u32>`. Envia o resultado pré-tokenizado para o `LlamaCppEngine`.
   - *Validação*: Assevera que o logit do último token retornado pela decodificação do buffer binário é matematicamente idêntico ao logit gerado pelo prefill clássico baseado em texto bruto (`RawText`).

2. **`test_gigatoken_vocab_self_healing`**:
   - *Ação*: Apaga ou renomeia temporariamente o arquivo `tokenizer.json` na pasta de testes. Executa a inicialização do `GigaTokenEncoder`.
   - *Validação*: Assevera que o arquivo `tokenizer_recovered.json` foi criado fisicamente no disco e que a tokenização de um snippet de código produz a sequência numérica idêntica de IDs de tokens do dicionário original.

3. **`test_gigatoken_throughput_benchmark`**:
   - *Ação*: Alimenta o `GigaTokenEncoder` com um arquivo de código pesado (mock do orchestrator.rs de 10KB).
   - *Validação*: Mede a latência de execução do parse na CPU Host e assevera que ela é estritamente menor do que **5 milissegundos** ($\le 5\text{ms}$).

4. **`test_vram_budget_math`**:
   - *Ação*: Calcula a pegada estática e dinâmica de memória sob a configuração de $32\text{k}$ tokens de contexto com quantização do KV Cache em `Q4_K`.
   - *Validação*: Assevera que a soma dos pesos estáticos do Qwen Coder com as páginas de KV Cache permanece abaixo do teto estrito de **5.5 GB** ($\le 5632\text{ MB}$).
