# 📊 SOULS LLM INVENTORY SUMMARY & TELEMETRY DOSSIER
**Data de Geração:** 2026-08-24 20:08:28 | **Banco SSOT:** `Z:\souls_mc\.souls_data\souls_heuristic_vault.db`

---

## 🖥️ RESUMO EXECUTIVO DE HARDWARE & CAPACIDADES DO HOST
- **Placa Gráfica (Target GPU):** NVIDIA GeForce RTX 2060m (6GB VRAM, Arquitetura Turing)
- **Aceleração Host:** CPU Intel Core i9 (AVX2 SIMD Acceleration) + Gateway Tokio Rust
- **Infradesign Bare-Metal:** C-FFI Zero-Garbage (`llama_cpp_2`), Offload Adaptativo de Camadas GPU (n_gpu_layers=99)
- **Limites Térmicos & Trava de Contexto:** Hard-Cap de 32k tokens na família Gemma (`cap_context_length_for_family`), Cache KV Assimétrico (F16 Keys / Q4_K ou Q8_0 Values)
- **Fórmula de Eficiência (Score E3):** $E3 = \frac{\text{Acurácia}^2}{\text{Latência Total (s)} + 0.001}$

---

## 📈 MÉTRICAS CONSOLIDADAS DAS LLMs PRINCIPAIS

| Total GGUF | LLMs Principais | Aprovados Tier 1 | Reprovados (Guilhotina) | Pendentes | Sidecars | Modelos Core (src-tauri) |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| 73 | 62 | 0 | 5 | 57 | 11 | 5 |

### 🏆 TABELA DE PERFORMANCE E RANKING DE MODELOS (TIER 1 / TIER 2)

| # | Nome do Modelo | Família | Quant | Tamanho | TTFT (ms) | TPOT (ms) | Acurácia | Score E3 | Status Tier 1 |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| 1 | `Qwen - Qwen3 (GGUF)` | qwen35 | Q4_K_S | 2.45 GB | 0.1 | 0.0 | 0.0% | **0.0000** | ❌ Reprovado (Guilhotina) |
| 2 | `Unsloth - Unsloth Nbtyw0Rt (GGUF)` | qwen35 | Q4_0 | 2.43 GB | 0.1 | 0.0 | 0.0% | **0.0000** | ❌ Reprovado (Guilhotina) |
| 3 | `Local - Bonsai 27B (GGUF)` | qwen35 | GGUF_CUSTOM | 3.54 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 4 | `Local - Bonsai 27B Dspark (GGUF)` | dspark | GGUF | 1.66 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 5 | `Local - Mamba Codestral 7B V0 (GGUF)` | llama | Q4_K_M | 4.07 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 6 | `Qwen - Qwen3 (GGUF)` | dflash | Q4_K_M | 363.78 MB | 0.1 | N/A | 0.0% | **0.0000** | ❌ Reprovado (Guilhotina) |
| 7 | `Qwen - Parable V2 4B Merged (GGUF)` | qwen3 | Q4_K_M | 2.33 GB | 0.1 | N/A | 0.0% | **0.0000** | ❌ Reprovado (Guilhotina) |
| 8 | `Microsoft - Fara1 (GGUF)` | qwen35 | Q4_K_M | 2.52 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 9 | `Local - Zamba2 2 (Q4_0)` | zamba2 | Q4_0 | 2.08 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 10 | `Local - Smollm2 360M 8k Lc100K Mix1 Ep2 (GGUF)` | llama | Q8_0 | 368.50 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 11 | `Local - Transformers (GGUF)` | llama | Q4_K_M | 100.57 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 12 | `Nimbus Labs - 4B (GGUF)` | qwen35 | Q4_K_M | 2.52 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 13 | `Local - Laguna XS 2 (GGUF)` | dflash | Q4_K_M | 297.63 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 14 | `Local - Neuralai Mamba K1 V3 Merged (GGUF)` | mamba | Q4_K_M | 85.74 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 15 | `Local - Mamba 790m Hf (GGUF)` | mamba | Q4_K_M | 459.26 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 16 | `Local - SmolLM3 3B (GGUF)` | smollm3 | Q5_K_M | 2.06 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 17 | `Microsoft - Fara 7B (GGUF)` | qwen2vl | GGUF_CUSTOM | 3.33 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 18 | `Unsloth - Unsloth O97Brpro (GGUF)` | llama | Q4_K_M | 100.57 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 19 | `Qwen - Qwen2 (GGUF)` | qwen2 | Q5_K_M | 2.07 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 20 | `Local - Gemma 4 E2B It Ultra Uncensored Heretic (GGUF)` | gemma4 | Q4_K_M | 3.19 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 21 | `Local - Bonsai 27B (GGUF)` | qwen35 | GGUF_CUSTOM | 3.54 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 22 | `Qwen - Deepseek R1 Distill 1 (GGUF)` | qwen2 | Q8_0 | 1.76 GB | 0.1 | N/A | 0.0% | **0.0000** | ❌ Reprovado (Guilhotina) |
| 23 | `Local - Liquidai LFM2 5 1 (GGUF)` | lfm2 | Q8_0 | 1.16 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 24 | `LMStudio - Community Nn (GGUF)` | nemotron_h | Q4_K_M | 2.64 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 25 | `Local - Phi-4 Mini Instruct (GGUF)` | phi3 | Q4_K_M | 2.32 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 26 | `Local - Phi-4 Mini Reasoning (GGUF)` | phi3 | Q4_K_M | 2.32 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 27 | `Local - Gemma 4 E2B (GGUF)` | gemma4 | Q4_K_M | 3.19 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 28 | `Local - Essentialai Rnj 1 Instruct (GGUF)` | gemma3 | Q4_K_M | 4.76 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 29 | `Local - Falcon3 Mamba 7B Instruct (GGUF)` | mamba | GGUF_CUSTOM | 3.05 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 30 | `Local - Synthagent SFT UI TARS 1 (GGUF)` | qwen2vl | GGUF_CUSTOM | 3.33 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 31 | `Local - Safetensors (GGUF)` | laguna | Q4_K_M | 14.69 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 32 | `Local - 7B (GGUF)` | hunyuan-dense | Q4_K_M | 4.31 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 33 | `Local - Phi-4 Mini Instruct (GGUF)` | phi3 | Q5_K_M | 2.65 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 34 | `Local - Mamba Codestral 7B V0 (GGUF)` | mamba2 | Q4_K_M | 3.86 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 35 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 36 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 37 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 38 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 39 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 40 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 41 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 42 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 43 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 44 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 45 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 46 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 47 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 48 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 49 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 50 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 51 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 52 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 53 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 54 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 55 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 56 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 57 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 58 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 59 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 60 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 61 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 62 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.00 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |

---

## 🧩 SEÇÃO 2: MÓDULOS AUXILIARES E SIDECARS (11)

| # | Nome do Módulo | Tipo de Sidecar | Tamanho | Caminho Físico |
| :--- | :--- | :---: | :---: | :--- |
| 1 | `Bonsai-27B-mmproj-BF16.gguf` | `VISION_PROJECTOR` | 888.01 MB | `C:\Users\rosas\.lmstudio\models\AI-Joe-git\Bonsai-27B-gguf\Bonsai-27B-mmproj-BF16.gguf` |
| 2 | `Qwen3.5-4B-Uncensored-HauhauCS-Aggressive-MTP-Q4_K_M.gguf` | `MTP_ADAPTER` | 2.60 GB | `C:\Users\rosas\.lmstudio\models\AIOpsInSpace\Qwen3.5-4B-Uncensored-HauhauCS-Aggressive-MTP\Qwen3.5-4B-Uncensored-HauhauCS-Aggressive-MTP-Q4_K_M.gguf` |
| 3 | `mmproj-microsoft.Fara1.5-4B.f16.gguf` | `VISION_PROJECTOR` | 641.27 MB | `C:\Users\rosas\.lmstudio\models\DevQuasar\microsoft.Fara1.5-4B-GGUF\mmproj-microsoft.Fara1.5-4B.f16.gguf` |
| 4 | `DeepSeek-V4-Pro-Qwen3.5-4B-MTP-Q4_K_S.gguf` | `MTP_ADAPTER` | 2.45 GB | `C:\Users\rosas\.lmstudio\models\Jackrong\DeepSeek-V4-Pro-Qwen3.5-4B-MTP-GGUF\DeepSeek-V4-Pro-Qwen3.5-4B-MTP-Q4_K_S.gguf` |
| 5 | `mmproj-F32.gguf` | `VISION_PROJECTOR` | 1.24 GB | `C:\Users\rosas\.lmstudio\models\Jackrong\DeepSeek-V4-Pro-Qwen3.5-4B-MTP-GGUF\mmproj-F32.gguf` |
| 6 | `mmproj-Nimbus-4B-F16.gguf` | `VISION_PROJECTOR` | 641.27 MB | `C:\Users\rosas\.lmstudio\models\Nimbus-Labs\Nimbus-4B-GGUF\mmproj-Nimbus-4B-F16.gguf` |
| 7 | `mmproj-microsoft_Fara-7B-f16.gguf` | `VISION_PROJECTOR` | 1.26 GB | `C:\Users\rosas\.lmstudio\models\bartowski\microsoft_Fara-7B-GGUF\mmproj-microsoft_Fara-7B-f16.gguf` |
| 8 | `gemma-4-E2B-it-mmproj-BF16.gguf` | `VISION_PROJECTOR` | 941.12 MB | `C:\Users\rosas\.lmstudio\models\llmfan46\gemma-4-E2B-it-ultra-uncensored-heretic-GGUF\gemma-4-E2B-it-mmproj-BF16.gguf` |
| 9 | `mmproj-Bonsai-27B-BF16.gguf` | `VISION_PROJECTOR` | 888.01 MB | `C:\Users\rosas\.lmstudio\models\lmstudio-community\Bonsai-27B-GGUF\mmproj-Bonsai-27B-BF16.gguf` |
| 10 | `mmproj-gemma-4-E2B-it-BF16.gguf` | `VISION_PROJECTOR` | 941.12 MB | `C:\Users\rosas\.lmstudio\models\lmstudio-community\gemma-4-E2B-it-GGUF\mmproj-gemma-4-E2B-it-BF16.gguf` |
| 11 | `ggml-model-i2_s.gguf` | `SPECIALIZED_QUANT` | 1.11 GB | `C:\Users\rosas\.lmstudio\models\microsoft\bitnet-b1.58-2B-4T-gguf\ggml-model-i2_s.gguf` |

---

## 📦 SEÇÃO 3: MODELOS EMBARCADOS & CORE INTERNOS (src-tauri/models) (5)

| # | Nome do Arquivo | Categoria | Formato | Tamanho | Descrição |
| :--- | :--- | :---: | :---: | :---: | :--- |
| 1 | `gliclass_multilang.onnx.data` | `MODEL_WEIGHTS` | `DATA` | 3.18 GB | Modelo ONNX de Classificação de Intenções / NER |
| 2 | `tokenizer.json` | `TOKENIZER_CONFIG` | `JSON` | 15.58 MB | Arquivo de Configuração / Tokenizer Core |
| 3 | `gliclass_multilang.onnx` | `MODEL_WEIGHTS` | `ONNX` | 4.03 MB | Modelo ONNX de Classificação de Intenções / NER |
| 4 | `tokenizer_recovered.json` | `TOKENIZER_CONFIG` | `JSON` | 0.01 MB | Arquivo de Configuração / Tokenizer Core |
| 5 | `tokenizer_config.json` | `TOKENIZER_CONFIG` | `JSON` | 0.00 MB | Arquivo de Configuração / Tokenizer Core |

---

## 📝 DETALHAMENTO E DOSSIÊ INDIVIDUAL DOS MODELOS

### 1. `Qwen - Qwen3 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\armandosds\qwen3.5-4b-agentic-coder-v4-i1-GGUF\qwen3.5-4b-agentic-coder-v4.i1-Q4_K_S.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `4B` | Contexto Máximo `262144` | Quant `Q4_K_S`
- **Telemetria:** TTFT `0.15 ms` | TPOT `0.01 ms` | Latência Média `1.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** PURGA RECOMENDADA DO SSD: Reprovado por falhas sintáticas ou latência incompatível com o hardware.

### 2. `Unsloth - Unsloth Nbtyw0Rt (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\stevenlearns\qwen3.5-4B-super-coder\qwen3.5-4B-super-coder.Q4_0.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `4B` | Contexto Máximo `262144` | Quant `Q4_0`
- **Telemetria:** TTFT `0.15 ms` | TPOT `0.01 ms` | Latência Média `1.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** PURGA RECOMENDADA DO SSD: Reprovado por falhas sintáticas ou latência incompatível com o hardware.

### 3. `Local - Bonsai 27B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AI-Joe-git\Bonsai-27B-gguf\Bonsai-27B-Q1_0.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `27B` | Contexto Máximo `262144` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Trilhas Cognitivas:** Vision VQA: `100%`
- **Módulos Anexados:** `Bonsai-27B-mmproj-BF16.gguf` (Pareado SQLite)
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 4. `Local - Bonsai 27B Dspark (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AI-Joe-git\Bonsai-27B-gguf\Bonsai-27B-dspark-Q4_1.gguf`
- **Metadados:** Família `dspark` | Parâmetros `27B` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Trilhas Cognitivas:** Vision VQA: `100%`
- **Módulos Anexados:** `Bonsai-27B-mmproj-BF16.gguf` (Pareado SQLite)
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 5. `Local - Mamba Codestral 7B V0 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Agnuxo\Mamba-Codestral-7B-Instruct_CODE_Python-Spanish_English_GGUF_4bit\unsloth.Q4_K_M.gguf`
- **Metadados:** Família `llama` | Parâmetros `7B` | Contexto Máximo `32768` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 6. `Qwen - Qwen3 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Anbeeld\Qwen3.5-4B-DFlash-GGUF\qwen35-4b-dflash-Q4_K_M.gguf`
- **Metadados:** Família `dflash` | Parâmetros `4B` | Contexto Máximo `262144` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.1 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** PURGA RECOMENDADA DO SSD: Reprovado por falhas sintáticas ou latência incompatível com o hardware.

### 7. `Qwen - Parable V2 4B Merged (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AnkitAI\Parable-Qwen3-4B-Claude-Fable-5-GGUF\Parable-Qwen3-4B-Claude-Fable-5-GGUF-Q4_K_M.gguf`
- **Metadados:** Família `qwen3` | Parâmetros `4B` | Contexto Máximo `40960` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.1 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** `Qwen3.5-4B-Uncensored-HauhauCS-Aggressive-MTP-Q4_K_M.gguf` (2.60 GB), `DeepSeek-V4-Pro-Qwen3.5-4B-MTP-Q4_K_S.gguf` (2.45 GB)
- **Veredito SOULS:** PURGA RECOMENDADA DO SSD: Reprovado por falhas sintáticas ou latência incompatível com o hardware.

### 8. `Microsoft - Fara1 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\DevQuasar\microsoft.Fara1.5-4B-GGUF\microsoft.Fara1.5-4B.f16.gguf.Q4_K_M.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `4B` | Contexto Máximo `262144` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Trilhas Cognitivas:** Vision VQA: `100%`
- **Módulos Anexados:** `mmproj-microsoft.Fara1.5-4B.f16.gguf` (Pareado SQLite)
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 9. `Local - Zamba2 2 (Q4_0)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\EchoLabs33\Zamba2-2.7B-Instruct-v2-GGUF\zamba2-2.7b-instruct-v2-q4_0.gguf`
- **Metadados:** Família `zamba2` | Parâmetros `2.7B` | Contexto Máximo `4096` | Quant `Q4_0`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 10. `Local - Smollm2 360M 8k Lc100K Mix1 Ep2 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\HuggingFaceTB\SmolLM2-360M-Instruct-GGUF\smollm2-360m-instruct-q8_0.gguf`
- **Metadados:** Família `llama` | Parâmetros `Unknown` | Contexto Máximo `8192` | Quant `Q8_0`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 11. `Local - Transformers (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Ma7ee7\SmolLM2-135M-Reasoning-5K-GGUF\SmolLM2-135M-Reasoning-5K-Q4_K_M.gguf`
- **Metadados:** Família `llama` | Parâmetros `Unknown` | Contexto Máximo `8192` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 12. `Nimbus Labs - 4B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Nimbus-Labs\Nimbus-4B-GGUF\Nimbus-4B-Q4_K_M.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `4B` | Contexto Máximo `262144` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Trilhas Cognitivas:** Vision VQA: `100%`
- **Módulos Anexados:** `mmproj-Nimbus-4B-F16.gguf` (Pareado SQLite)
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 13. `Local - Laguna XS 2 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\RespectMathias\Laguna-XS-2.1-DSpark-GGUF\Laguna-XS-2.1-DSpark-Q4_K_M.gguf`
- **Metadados:** Família `dflash` | Parâmetros `Unknown` | Contexto Máximo `262144` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 14. `Local - Neuralai Mamba K1 V3 Merged (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Subject-Emu-5259\NeuralAI-Mamba-K1\NeuralAI-Mamba-K1-v3.Q4_K_M.gguf`
- **Metadados:** Família `mamba` | Parâmetros `Unknown` | Contexto Máximo `1048576` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 15. `Local - Mamba 790m Hf (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Subject-Emu-5259\NeuralAI-Mamba-K2\mamba-790m-hf.Q4_K_M.gguf`
- **Metadados:** Família `mamba` | Parâmetros `Unknown` | Contexto Máximo `1048576` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 16. `Local - SmolLM3 3B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\bartowski\HuggingFaceTB_SmolLM3-3B-GGUF\HuggingFaceTB_SmolLM3-3B-Q5_K_M.gguf`
- **Metadados:** Família `smollm3` | Parâmetros `3B` | Contexto Máximo `65536` | Quant `Q5_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 17. `Microsoft - Fara 7B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\bartowski\microsoft_Fara-7B-GGUF\microsoft_Fara-7B-IQ3_M.gguf`
- **Metadados:** Família `qwen2vl` | Parâmetros `7B` | Contexto Máximo `128000` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Trilhas Cognitivas:** Vision VQA: `100%`
- **Módulos Anexados:** `mmproj-microsoft_Fara-7B-f16.gguf` (Pareado SQLite)
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 18. `Unsloth - Unsloth O97Brpro (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\hypaai\Hypa-SmolLM-135M-Instruct-GGUF\smollm-135m-instruct.Q4_K_M.gguf`
- **Metadados:** Família `llama` | Parâmetros `Unknown` | Contexto Máximo `2048` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 19. `Qwen - Qwen2 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\katanemo\Arch-Function-Chat-3B.gguf\Arch-Function-Chat-3B-Q5_K_M.gguf`
- **Metadados:** Família `qwen2` | Parâmetros `3B` | Contexto Máximo `32768` | Quant `Q5_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 20. `Local - Gemma 4 E2B It Ultra Uncensored Heretic (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\llmfan46\gemma-4-E2B-it-ultra-uncensored-heretic-GGUF\gemma-4-E2B-it-ultra-uncensored-heretic-Q4_K_M.gguf`
- **Metadados:** Família `gemma4` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Trilhas Cognitivas:** Vision VQA: `100%`
- **Módulos Anexados:** `gemma-4-E2B-it-mmproj-BF16.gguf` (Pareado SQLite)
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 21. `Local - Bonsai 27B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\Bonsai-27B-GGUF\Bonsai-27B-Q1_0.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `27B` | Contexto Máximo `262144` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Trilhas Cognitivas:** Vision VQA: `100%`
- **Módulos Anexados:** `mmproj-Bonsai-27B-BF16.gguf` (Pareado SQLite)
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 22. `Qwen - Deepseek R1 Distill 1 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\DeepSeek-R1-Distill-Qwen-1.5B-GGUF\DeepSeek-R1-Distill-Qwen-1.5B-Q8_0.gguf`
- **Metadados:** Família `qwen2` | Parâmetros `1.5B` | Contexto Máximo `131072` | Quant `Q8_0`
- **Telemetria:** TTFT `0.1 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** PURGA RECOMENDADA DO SSD: Reprovado por falhas sintáticas ou latência incompatível com o hardware.

### 23. `Local - Liquidai LFM2 5 1 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\LFM2.5-1.2B-Instruct-GGUF\LFM2.5-1.2B-Instruct-Q8_0.gguf`
- **Metadados:** Família `lfm2` | Parâmetros `1.2B` | Contexto Máximo `128000` | Quant `Q8_0`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 24. `LMStudio - Community Nn (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\NVIDIA-Nemotron-3-Nano-4B-GGUF\NVIDIA-Nemotron-3-Nano-4B-Q4_K_M.gguf`
- **Metadados:** Família `nemotron_h` | Parâmetros `4B` | Contexto Máximo `1048576` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 25. `Local - Phi-4 Mini Instruct (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\Phi-4-mini-instruct-GGUF\Phi-4-mini-instruct-Q4_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 26. `Local - Phi-4 Mini Reasoning (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\Phi-4-mini-reasoning-GGUF\Phi-4-mini-reasoning-Q4_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 27. `Local - Gemma 4 E2B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf`
- **Metadados:** Família `gemma4` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Trilhas Cognitivas:** Vision VQA: `100%`
- **Módulos Anexados:** `mmproj-gemma-4-E2B-it-BF16.gguf` (Pareado SQLite)
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 28. `Local - Essentialai Rnj 1 Instruct (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\rnj-1-instruct-GGUF\rnj-1-instruct-Q4_K_M.gguf`
- **Metadados:** Família `gemma3` | Parâmetros `Unknown` | Contexto Máximo `32768` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 29. `Local - Falcon3 Mamba 7B Instruct (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\mradermacher\Falcon3-Mamba-7B-Instruct-i1-GGUF\Falcon3-Mamba-7B-Instruct.i1-IQ3_M.gguf`
- **Metadados:** Família `mamba` | Parâmetros `7B` | Contexto Máximo `1048576` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 30. `Local - Synthagent SFT UI TARS 1 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\mradermacher\SynthAgent-SFT-UI-TARS-1.5-7B-i1-GGUF\SynthAgent-SFT-UI-TARS-1.5-7B.i1-IQ3_M.gguf`
- **Metadados:** Família `qwen2vl` | Parâmetros `7B` | Contexto Máximo `128000` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 31. `Local - Safetensors (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\mudler\Laguna-XS-2.1-APEX-GGUF\Laguna-XS-2.1-APEX-I-Compact.gguf`
- **Metadados:** Família `laguna` | Parâmetros `Unknown` | Contexto Máximo `262144` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 32. `Local - 7B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\tencent\Hy-MT2-7B-GGUF\Hy-MT2-7B-Q4_K_M.gguf`
- **Metadados:** Família `hunyuan-dense` | Parâmetros `7B` | Contexto Máximo `262144` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 33. `Local - Phi-4 Mini Instruct (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\unsloth\Phi-4-mini-instruct-GGUF\Phi-4-mini-instruct-Q5_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q5_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 34. `Local - Mamba Codestral 7B V0 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\viniciusianni\Mamba-Codestral-7B-v0.1-Q4_K_M-GGUF\mamba-codestral-7b-v0.1-q4_k_m.gguf`
- **Metadados:** Família `mamba2` | Parâmetros `7B` | Contexto Máximo `1048576` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 35. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmp21Zk0L\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 36. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmp4OBef7\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 37. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpAFSxx0\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 38. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpBVDKLG\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 39. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpCB6RRb\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 40. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpG1LDH7\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 41. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpH8hWWq\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 42. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpHSpijG\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 43. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpK5N97y\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 44. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpLYM05l\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 45. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpN9CLVt\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 46. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpOqOmxk\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 47. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpRqEinQ\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 48. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpUUPPll\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 49. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpVaOuNu\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 50. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpVxzu4V\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 51. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpWpBZNv\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 52. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpXANvxM\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 53. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpXFt2nc\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 54. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpYyGTkP\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 55. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpaHeuxU\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 56. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpejMGrG\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 57. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpgdQhZI\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 58. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpi6E6Zx\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 59. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpjWYfpJ\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 60. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpkIUTgw\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 61. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmporff6Y\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 62. `Local - Stress Test Model (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpqxjlzs\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

---
*Fim do Dossiê de Inventário SOULS v4. Gerado automaticamente via `souls_llms_inventory_viewer.py`.*