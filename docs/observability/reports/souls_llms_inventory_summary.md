# 📊 SOULS LLM INVENTORY SUMMARY & TELEMETRY DOSSIER
**Data de Geração:** 2026-08-05 | **Banco SSOT:** `Z:\souls_mc\.souls_data\souls_heuristic_vault.db`

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
| 24 | 24 | 24 | 0 | 0 | 0 |

### 🏆 TABELA DE PERFORMANCE E RANKING DE MODELOS (TIER 1 / TIER 2)

| # | Nome do Modelo | Família | Quant | Tamanho | TTFT (ms) | TPOT (ms) | Acurácia JSON | Score E3 | Status Tier 1 |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| 1 | `Local - Bonsai 27B Dspark (GGUF)` | dspark | GGUF | 1.66 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 2 | `Local - Bonsai 27B (GGUF)` | clip | GGUF_CUSTOM | 888.01 MB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 3 | `Local - Bonsai 27B (GGUF)` | qwen35 | GGUF_CUSTOM | 3.54 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 4 | `Qwen - Parable V2 4B Merged (GGUF)` | qwen3 | Q4_K_M | 2.33 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 5 | `Local - SmolLM3 3B (GGUF)` | smollm3 | Q5_K_M | 2.06 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 6 | `Microsoft - Fara 7B (GGUF)` | qwen2vl | GGUF_CUSTOM | 3.33 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 7 | `Microsoft - Fara 7B (GGUF)` | clip | F16 | 1.26 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 8 | `Local - Zamba2 2 (Q4_0)` | zamba2 | Q4_0 | 2.08 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 9 | `Local - Smollm2 360M 8k Lc100K Mix1 Ep2 (GGUF)` | llama | Q8_0 | 368.50 MB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 10 | `Qwen - Qwen2 (GGUF)` | qwen2 | Q5_K_M | 2.07 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 11 | `Local - Gemma 4 E2B It (GGUF)` | clip | GGUF_CUSTOM | 941.12 MB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 12 | `Local - Gemma 4 E2B It Ultra Uncensored Heretic (GGUF)` | gemma4 | Q4_K_M | 3.19 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 13 | `Qwen - Deepseek R1 Distill 1 (GGUF)` | qwen2 | Q8_0 | 1.76 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 14 | `Local - Gemma 4 E2B (GGUF)` | gemma4 | Q4_K_M | 3.19 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 15 | `Local - Gemma 4 E2B (GGUF)` | clip | GGUF_CUSTOM | 941.12 MB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 16 | `LMStudio - Community Nn (GGUF)` | nemotron_h | Q4_K_M | 2.64 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 17 | `Local - Phi-4 Mini Instruct (GGUF)` | phi3 | Q4_K_M | 2.32 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 18 | `Local - Bitnet2b (GGUF)` | bitnet-b1.58 | GGUF_CUSTOM | 1.11 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 19 | `Local - Falcon3 Mamba 7B Instruct (GGUF)` | mamba | GGUF_CUSTOM | 3.05 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 20 | `Local - Synthagent SFT UI TARS 1 (GGUF)` | qwen2vl | GGUF_CUSTOM | 3.33 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 21 | `Nimbus Labs - 4b (GGUF)` | clip | F16 | 641.27 MB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 22 | `Nimbus Labs - 4B (GGUF)` | qwen35 | Q4_K_M | 2.52 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 23 | `Local - 7B (GGUF)` | hunyuan-dense | Q4_K_M | 4.31 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |
| 24 | `Local - Phi-4 Mini Instruct (GGUF)` | phi3 | Q5_K_M | 2.65 GB | N/A | N/A | 100.0% | **1000.0000** | ✅ Aprovado (Tier 1) |

---

## 🧩 MÓDULOS AUXILIARES E SIDECARS (VISÃO / MTP)

| # | Nome do Módulo | Tipo de Sidecar | Tamanho | Caminho Físico |
| :--- | :--- | :---: | :---: | :--- |
| - | Nenhum módulo auxiliar encontrado | - | - | - |

---

## 📝 DETALHAMENTO E DOSSIÊ INDIVIDUAL DOS MODELOS

### 1. `Local - Bonsai 27B Dspark (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AI-Joe-git\Bonsai-27B-gguf\Bonsai-27B-dspark-Q4_1.gguf`
- **Metadados:** Família `dspark` | Parâmetros `27B` | Contexto Máximo `4096` | Quant `GGUF`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 2. `Local - Bonsai 27B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AI-Joe-git\Bonsai-27B-gguf\Bonsai-27B-mmproj-BF16.gguf`
- **Metadados:** Família `clip` | Parâmetros `27B` | Contexto Máximo `4096` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 3. `Local - Bonsai 27B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AI-Joe-git\Bonsai-27B-gguf\Bonsai-27B-Q1_0.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `27B` | Contexto Máximo `262144` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 4. `Qwen - Parable V2 4B Merged (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AnkitAI\Parable-Qwen3-4B-Claude-Fable-5-GGUF\Parable-Qwen3-4B-Claude-Fable-5-GGUF-Q4_K_M.gguf`
- **Metadados:** Família `qwen3` | Parâmetros `4B` | Contexto Máximo `40960` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 5. `Local - SmolLM3 3B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\bartowski\HuggingFaceTB_SmolLM3-3B-GGUF\HuggingFaceTB_SmolLM3-3B-Q5_K_M.gguf`
- **Metadados:** Família `smollm3` | Parâmetros `3B` | Contexto Máximo `65536` | Quant `Q5_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 6. `Microsoft - Fara 7B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\bartowski\microsoft_Fara-7B-GGUF\microsoft_Fara-7B-IQ3_M.gguf`
- **Metadados:** Família `qwen2vl` | Parâmetros `7B` | Contexto Máximo `128000` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 7. `Microsoft - Fara 7B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\bartowski\microsoft_Fara-7B-GGUF\mmproj-microsoft_Fara-7B-f16.gguf`
- **Metadados:** Família `clip` | Parâmetros `7B` | Contexto Máximo `4096` | Quant `F16`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 8. `Local - Zamba2 2 (Q4_0)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\EchoLabs33\Zamba2-2.7B-Instruct-v2-GGUF\zamba2-2.7b-instruct-v2-q4_0.gguf`
- **Metadados:** Família `zamba2` | Parâmetros `2.7B` | Contexto Máximo `4096` | Quant `Q4_0`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 9. `Local - Smollm2 360M 8k Lc100K Mix1 Ep2 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\HuggingFaceTB\SmolLM2-360M-Instruct-GGUF\smollm2-360m-instruct-q8_0.gguf`
- **Metadados:** Família `llama` | Parâmetros `Unknown` | Contexto Máximo `8192` | Quant `Q8_0`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 10. `Qwen - Qwen2 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\katanemo\Arch-Function-Chat-3B.gguf\Arch-Function-Chat-3B-Q5_K_M.gguf`
- **Metadados:** Família `qwen2` | Parâmetros `3B` | Contexto Máximo `32768` | Quant `Q5_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 11. `Local - Gemma 4 E2B It (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\llmfan46\gemma-4-E2B-it-ultra-uncensored-heretic-GGUF\gemma-4-E2B-it-mmproj-BF16.gguf`
- **Metadados:** Família `clip` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 12. `Local - Gemma 4 E2B It Ultra Uncensored Heretic (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\llmfan46\gemma-4-E2B-it-ultra-uncensored-heretic-GGUF\gemma-4-E2B-it-ultra-uncensored-heretic-Q4_K_M.gguf`
- **Metadados:** Família `gemma4` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 13. `Qwen - Deepseek R1 Distill 1 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\DeepSeek-R1-Distill-Qwen-1.5B-GGUF\DeepSeek-R1-Distill-Qwen-1.5B-Q8_0.gguf`
- **Metadados:** Família `qwen2` | Parâmetros `1.5B` | Contexto Máximo `131072` | Quant `Q8_0`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 14. `Local - Gemma 4 E2B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf`
- **Metadados:** Família `gemma4` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 15. `Local - Gemma 4 E2B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\gemma-4-E2B-it-GGUF\mmproj-gemma-4-E2B-it-BF16.gguf`
- **Metadados:** Família `clip` | Parâmetros `Unknown` | Contexto Máximo `4096` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 16. `LMStudio - Community Nn (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\NVIDIA-Nemotron-3-Nano-4B-GGUF\NVIDIA-Nemotron-3-Nano-4B-Q4_K_M.gguf`
- **Metadados:** Família `nemotron_h` | Parâmetros `4B` | Contexto Máximo `1048576` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 17. `Local - Phi-4 Mini Instruct (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\Phi-4-mini-instruct-GGUF\Phi-4-mini-instruct-Q4_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 18. `Local - Bitnet2b (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\microsoft\bitnet-b1.58-2B-4T-gguf\ggml-model-i2_s.gguf`
- **Metadados:** Família `bitnet-b1.58` | Parâmetros `2B` | Contexto Máximo `4096` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 19. `Local - Falcon3 Mamba 7B Instruct (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\mradermacher\Falcon3-Mamba-7B-Instruct-i1-GGUF\Falcon3-Mamba-7B-Instruct.i1-IQ3_M.gguf`
- **Metadados:** Família `mamba` | Parâmetros `7B` | Contexto Máximo `1048576` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 20. `Local - Synthagent SFT UI TARS 1 (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\mradermacher\SynthAgent-SFT-UI-TARS-1.5-7B-i1-GGUF\SynthAgent-SFT-UI-TARS-1.5-7B.i1-IQ3_M.gguf`
- **Metadados:** Família `qwen2vl` | Parâmetros `7B` | Contexto Máximo `128000` | Quant `GGUF_CUSTOM`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 21. `Nimbus Labs - 4b (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Nimbus-Labs\Nimbus-4B-GGUF\mmproj-Nimbus-4B-F16.gguf`
- **Metadados:** Família `clip` | Parâmetros `4B` | Contexto Máximo `4096` | Quant `F16`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 22. `Nimbus Labs - 4B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Nimbus-Labs\Nimbus-4B-GGUF\Nimbus-4B-Q4_K_M.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `4B` | Contexto Máximo `262144` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 23. `Local - 7B (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\tencent\Hy-MT2-7B-GGUF\Hy-MT2-7B-Q4_K_M.gguf`
- **Metadados:** Família `hunyuan-dense` | Parâmetros `7B` | Contexto Máximo `262144` | Quant `Q4_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

### 24. `Local - Phi-4 Mini Instruct (GGUF)`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\unsloth\Phi-4-mini-instruct-GGUF\Phi-4-mini-instruct-Q5_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` | Quant `Q5_K_M`
- **Telemetria:** TTFT `0.0 ms` | TPOT `0.0 ms` | Latência Média `0.00 ms` | Acurácia `100.0%` | **Score E3 `1000.0`**
- **Módulos Anexados:** Nenhum
- **Veredito SOULS:** RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta.

---
*Fim do Dossiê de Inventário SOULS v4. Gerado automaticamente via `souls_llms_inventory_viewer.py`.*