# 📊 SOULS LLM INVENTORY SUMMARY & TELEMETRY DOSSIER
**Data de Geração:** 2026-07-30 | **Banco SSOT:** `Z:\souls_mc\.souls_data\souls_heuristic_vault.db`

---

## 🖥️ RESUMO EXECUTIVO DE HARDWARE & CAPACIDADES DO HOST
- **Placa Gráfica (Target GPU):** NVIDIA GeForce RTX 2060m (6GB VRAM, Arquitetura Turing)
- **Aceleração Host:** CPU Intel Core i9 (AVX2 SIMD Acceleration) + Gateway Tokio Rust
- **Infradesign Bare-Metal:** C-FFI Zero-Garbage (`llama_cpp_2`), Offload Adaptativo de Camadas GPU (n_gpu_layers=99)
- **Limites Térmicos & Trava de Contexto:** Hard-Cap de 32k tokens na família Gemma (`cap_context_length_for_family`), Cache KV Assimétrico (F16 Keys / Q4_K ou Q8_0 Values)
- **Fórmula de Eficiência (Score E3):** $E3 = \frac{\text{Acurácia}^2}{\text{Latência Total (s)} + 0.001}$

---

## 📈 METRICAS CONSOLIDADAS DAS LLMs PRINCIPAIS

| Total GGUF | LLMs Principais | Aprovados Tier 1 | Reprovados (Guilhotina) | Pendentes | Sidecars |
| :---: | :---: | :---: | :---: | :---: | :---: |
| 8 | 8 | 0 | 0 | 8 | 0 |

### 🏆 TABELA DE PERFORMANCE E RANKING DE MODELOS (TIER 1 / TIER 2)

| # | Nome do Modelo | Família | Quant | Tamanho | TTFT (ms) | TPOT (ms) | Acurácia JSON | Score E3 | Status Tier 1 |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| 1 | `Qwen - Parable V2 4B Merged (GGUF)` | qwen3 | Q4_K_M | 2.33 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 2 | `Local - Smollm2 360M 8k Lc100K Mix1 Ep2 (GGUF)` | llama | Q8_0 | 368.50 MB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 3 | `Qwen - Qwen2 (GGUF)` | qwen2 | Q5_K_M | 2.07 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 4 | `Local - Gemma 4 E2B It Ultra Uncensored Heretic (GGUF)` | gemma4 | Q4_K_M | 3.19 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 5 | `Qwen - Deepseek R1 Distill 1 (GGUF)` | qwen2 | Q8_0 | 1.76 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 6 | `Local - Gemma 4 E2B (GGUF)` | gemma4 | Q4_K_M | 3.19 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 7 | `Local - Phi-4 Mini Instruct (GGUF)` | phi3 | Q4_K_M | 2.32 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |
| 8 | `Local - Phi-4 Mini Instruct (GGUF)` | phi3 | Q5_K_M | 2.65 GB | N/A | N/A | 0.0% | **0.0000** | ⏳ Pendente |

---

## 🧩 MÓDULOS AUXILIARES E SIDECARS (VISÃO / MTP)

| # | Nome do Módulo | Tipo de Sidecar | Tamanho | Caminho Físico |
| :--- | :--- | :---: | :---: | :--- |
| - | Nenhum módulo auxiliar encontrado | - | - | - |

---

## 📝 DETALHAMENTO E DOSSIÊ INDIVIDUAL DOS MODELOS

### 1. `Qwen - Parable V2 4B Merged (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AnkitAI\Parable-Qwen3-4B-Claude-Fable-5-GGUF\Parable-Qwen3-4B-Claude-Fable-5-GGUF-Q4_K_M.gguf`
- **Metadados:** Família `qwen3` | Parâmetros `4B` | Contexto Máximo `40960` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 2. `Local - Smollm2 360M 8k Lc100K Mix1 Ep2 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\HuggingFaceTB\SmolLM2-360M-Instruct-GGUF\smollm2-360m-instruct-q8_0.gguf`
- **Metadados:** Família `llama` | Parâmetros `Unknown` | Contexto Máximo `8192` | Quant `Q8_0`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 3. `Qwen - Qwen2 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\katanemo\Arch-Function-Chat-3B.gguf\Arch-Function-Chat-3B-Q5_K_M.gguf`
- **Metadados:** Família `qwen2` | Parâmetros `3B` | Contexto Máximo `32768` | Quant `Q5_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 4. `Local - Gemma 4 E2B It Ultra Uncensored Heretic (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\llmfan46\gemma-4-E2B-it-ultra-uncensored-heretic-GGUF\gemma-4-E2B-it-ultra-uncensored-heretic-Q4_K_M.gguf`
- **Metadados:** Família `gemma4` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 5. `Qwen - Deepseek R1 Distill 1 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\DeepSeek-R1-Distill-Qwen-1.5B-GGUF\DeepSeek-R1-Distill-Qwen-1.5B-Q8_0.gguf`
- **Metadados:** Família `qwen2` | Parâmetros `1.5B` | Contexto Máximo `131072` | Quant `Q8_0`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 6. `Local - Gemma 4 E2B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf`
- **Metadados:** Família `gemma4` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 7. `Local - Phi-4 Mini Instruct (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\Phi-4-mini-instruct-GGUF\Phi-4-mini-instruct-Q4_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

### 8. `Local - Phi-4 Mini Instruct (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\unsloth\Phi-4-mini-instruct-GGUF\Phi-4-mini-instruct-Q5_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q5_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `0.0%` | **Score E3 `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência.

---
*Fim do Dossiê de Inventário SOULS v4. Gerado automaticamente via `souls_llms_inventory_viewer.py`.*