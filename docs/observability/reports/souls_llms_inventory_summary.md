# 📊 SOULS SILICON OBSERVABILITY & LLM INVENTORY DOSSIER (V5)
**Data de Geração:** 2026-08-29 03:34:16 | **Banco SSOT:** `Z:\souls_mc\.souls_data\souls_heuristic_vault.db`

---

## 🖥️ 1. RESUMO EXECUTIVO DE SILÍCIO & GOVERNANÇA BARE-METAL
- **Aceleração Gráfica (Target GPU):** NVIDIA GeForce RTX 2060m (6GB VRAM GDDR6, Arquitetura Turing sm_75)
- **Aceleração Host (CPU):** Intel Core i9 (AVX2 SIMD AOT) + Gateway Tokio Rust
- **Matriz de Motores:**
  - `ik_llama_vanguard`: TurboQuant com V-Cache 4-bit, FlashAttention O(1) e LoRA residual.
  - `llama_upstream`: Binding oficial llama.cpp 2026 para arquiteturas Phi-4, Nemotron, LFM e Mamba GGUF.
  - `mistral_rs`: Runtime bare-metal especializado em State Space Models (SSM/Mamba).
  - `llama_cpp4`: Motor puro de CPU AVX2 para calibração, logit probing e fallback de sensor.
- **Eficiência FinOps ($E^3$ Score):** $E^3 = \frac{\text{Acurácia}^2}{\text{Latência Média (s)} + 0.001}$

---

## 📈 2. MÉTRICAS GLOBAIS DO INVENTÁRIO DE MODELOS

| Total GGUF | LLMs Principais | Aprovados Tier 1 | Reprovados/Quarentena | Pendentes | Sidecars | Modelos Core (src-tauri) |
| :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **85** | **74** | **20** | **0** | **54** | **11** | **5** |

### ⚡ Telemetria Agregada em Produção (`telemetry_logs`)
- **Execuções Registradas:** `186` chamadas
- **Tokens de Entrada:** `15.3k` | **Tokens de Saída:** `27.1k`
- **Custo FinOps Acumulado:** `$0.000000 USD` | **Latência Média:** `24199.7 ms`

---

## 🏆 3. LEADERBOARD POR TIER & MATRIZ DE MOTORES

### 🎯 Tier 0 (Bootstrap & CPU Sanity)

| # | Modelo | Família | Quant | Tamanho | Motor Campeão | TTFT (ms) | TPOT (ms) | TPS | VRAM Pico | Score E³ | Status |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| 1 | `gliclass_multilang` | gliclass | ONNX-F16 | 4.0 MB | `ort_scorer` | 48.9 | 4.5 | 224.7 | 40 MB | **607.6446** | ✅ Aprovado (Campeão) |
| 2 | `Local - Transformers (GGUF)` | llama | Q4_K_M | 100.6 MB | `ik_llama_vanguard` | 954.6 | 12.9 | 77.6 | 0 MB | **581.3801** | ✅ Aprovado (Campeão) |
| 3 | `Local - Smollm2 360M 8k Lc100K Mix1 Ep2 (GGUF)` | llama | Q8_0 | 368.5 MB | `ik_llama_vanguard` | 579.1 | 14.2 | 70.7 | 0 MB | **550.5181** | ✅ Aprovado (Campeão) |
| 4 | `Unsloth - Unsloth O97Brpro (GGUF)` | llama | Q4_K_M | 100.6 MB | `ik_llama_vanguard` | 153.6 | 14.5 | 68.8 | 0 MB | **529.3160** | ✅ Aprovado (Campeão) |
| 5 | `Local - Neuralai Mamba K1 V3 Merged (GGUF)` | mamba | Q4_K_M | 85.7 MB | `pulp_lele` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 6 | `Local - Mamba 790m Hf (GGUF)` | mamba | Q4_K_M | 459.3 MB | `pulp_lele` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 7 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 8 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 9 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 10 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 11 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 12 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 13 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 14 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 15 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 16 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 17 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 18 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 19 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 20 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 21 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 22 | `Local - Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 23 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 24 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 25 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 26 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 27 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 28 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 29 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 30 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 31 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 32 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 33 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 34 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 35 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 36 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 37 | `Local - Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 38 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 39 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 40 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 41 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 42 | `Local - Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 43 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 44 | `Local - Stress Test Model (GGUF)` | Generic | GGUF | 0.0 MB | `llama_cpp` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
### 🎯 Tier 0.5 (Sensor Epistêmico)

| # | Modelo | Família | Quant | Tamanho | Motor Campeão | TTFT (ms) | TPOT (ms) | TPS | VRAM Pico | Score E³ | Status |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| 1 | `Qwen - Deepseek R1 Distill 1 (GGUF)` | qwen2 | Q8_0 | 1.76 GB | `ik_llama_vanguard` | 1010.0 | 11.8 | 84.5 | 0 MB | **633.7418** | ✅ Aprovado (Campeão) |
| 2 | `Local - Liquidai LFM2 5 1 (GGUF)` | lfm2 | Q8_0 | 1.16 GB | `llama_upstream` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
### 🎯 Tier 1 (Live Chat & Master)

| # | Modelo | Família | Quant | Tamanho | Motor Campeão | TTFT (ms) | TPOT (ms) | TPS | VRAM Pico | Score E³ | Status |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| 1 | `Qwen - Qwen2 (GGUF)` | qwen2 | Q5_K_M | 2.07 GB | `ik_llama_vanguard` | 734.6 | 30.8 | 32.5 | 2052 MB | **15.2879** | ✅ Aprovado (Campeão) |
| 2 | `Unsloth - Unsloth Nbtyw0Rt (GGUF)` | qwen35 | Q4_0 | 2.43 GB | `ik_llama_vanguard` | 3582.2 | 42.0 | 23.8 | 2402 MB | **7.3045** | ✅ Aprovado (Campeão) |
| 3 | `Local - Mamba Codestral 7B V0 (GGUF)` | llama | Q4_K_M | 4.07 GB | `pulp_lele` | 2591.2 | 30.4 | 32.9 | 3998 MB | **6.1687** | ✅ Aprovado (Campeão) |
| 4 | `Local - SmolLM3 3B (GGUF)` | smollm3 | Q5_K_M | 2.06 GB | `ik_llama_vanguard` | 1805.6 | 71.0 | 14.1 | 2042 MB | **3.8684** | ✅ Aprovado (Campeão) |
| 5 | `Qwen - Qwen3 (GGUF)` | qwen35 | Q4_K_S | 2.45 GB | `ik_llama_vanguard` | 1672.3 | 88.5 | 11.3 | 2420 MB | **3.6409** | ✅ Aprovado (Campeão) |
| 6 | `Qwen - Parable V2 4B Merged (GGUF)` | qwen3 | Q4_K_M | 2.33 GB | `ik_llama_vanguard` | 1267.3 | 81.7 | 12.2 | 2299 MB | **2.0194** | ✅ Aprovado (Campeão) |
| 7 | `Local - Zamba2 2 (Q4_0)` | zamba2 | Q4_0 | 2.08 GB | `pulp_lele` | N/A | N/A | N/A | 2055 MB | **0.0000** | ⏳ Pendente |
| 8 | `model` | safetensors | F16 | 3.18 GB | `mistral_rs` | N/A | N/A | N/A | 3131 MB | **0.0000** | ⏳ Pendente |
| 9 | `LMStudio - Community Nn (GGUF)` | nemotron_h | Q4_K_M | 2.64 GB | `llama_upstream` | N/A | N/A | N/A | 2606 MB | **0.0000** | ⏳ Pendente |
| 10 | `Local - Phi-4 Mini Instruct (GGUF)` | phi3 | Q4_K_M | 2.32 GB | `llama_upstream` | N/A | N/A | N/A | 2294 MB | **0.0000** | ⏳ Pendente |
| 11 | `Local - Phi-4 Mini Reasoning (GGUF)` | phi3 | Q4_K_M | 2.32 GB | `llama_upstream` | N/A | N/A | N/A | 2294 MB | **0.0000** | ⏳ Pendente |
| 12 | `Local - Falcon3 Mamba 7B Instruct (GGUF)` | mamba | GGUF_CUSTOM | 3.05 GB | `pulp_lele` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 13 | `Local - 7B (GGUF)` | hunyuan-dense | Q4_K_M | 4.31 GB | `llama_upstream` | N/A | N/A | N/A | 4171 MB | **0.0000** | ⏳ Pendente |
| 14 | `Local - Phi-4 Mini Instruct (GGUF)` | phi3 | Q5_K_M | 2.65 GB | `llama_upstream` | N/A | N/A | N/A | 2616 MB | **0.0000** | ⏳ Pendente |
| 15 | `Local - Mamba Codestral 7B V0 (GGUF)` | mamba2 | Q4_K_M | 3.86 GB | `pulp_lele` | N/A | N/A | N/A | 3794 MB | **0.0000** | ⏳ Pendente |
### 🎯 Tier 2 (Background Agent & MoE Híbrido)

| # | Modelo | Família | Quant | Tamanho | Motor Campeão | TTFT (ms) | TPOT (ms) | TPS | VRAM Pico | Score E³ | Status |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| 1 | `Local - Safetensors (GGUF)` | laguna | Q4_K_M | 14.69 GB | `ik_llama_vanguard` | 5832.7 | 833.2 | 1.2 | 4173 MB | **0.2156** | ✅ Aprovado (Campeão) |
| 2 | `Local - Essentialai Rnj 1 Instruct (GGUF)` | gemma3 | Q4_K_M | 4.76 GB | `llama_upstream` | N/A | N/A | N/A | 4151 MB | **0.0000** | ⏳ Pendente |
### 🎯 Tier 3 (Vision & Multimodal VLM)

| # | Modelo | Família | Quant | Tamanho | Motor Campeão | TTFT (ms) | TPOT (ms) | TPS | VRAM Pico | Score E³ | Status |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| 1 | `Local - Gemma 4 E2B (GGUF)` | gemma4 | Q4_K_M | 3.19 GB | `ik_llama_vanguard` | 1100.3 | 25.9 | 38.6 | 3142 MB | **9.7903** | ✅ Aprovado (Campeão) |
| 2 | `Local - Gemma 4 E2B It Ultra Uncensored Heretic (GGUF)` | gemma4 | Q4_K_M | 3.19 GB | `ik_llama_vanguard` | 1007.5 | 27.5 | 36.3 | 3142 MB | **9.6354** | ✅ Aprovado (Campeão) |
| 3 | `Microsoft - Fara 7B (GGUF)` | qwen2vl | GGUF_CUSTOM | 3.33 GB | `ik_llama_vanguard` | 2559.3 | 71.4 | 14.0 | 3274 MB | **2.2145** | ✅ Aprovado (Campeão) |
| 4 | `Local - Synthagent SFT UI TARS 1 (GGUF)` | qwen2vl | GGUF_CUSTOM | 3.33 GB | `ik_llama_vanguard` | 2312.2 | 54.7 | 18.3 | 3274 MB | **2.1889** | ✅ Aprovado (Campeão) |
| 5 | `Microsoft - Fara1 (GGUF)` | qwen35 | Q4_K_M | 2.52 GB | `ik_llama_vanguard` | 1878.2 | 85.4 | 11.7 | 2490 MB | **2.0768** | ✅ Aprovado (Campeão) |
| 6 | `Nimbus Labs - 4B (GGUF)` | qwen35 | Q4_K_M | 2.52 GB | `ik_llama_vanguard` | 1469.7 | 97.1 | 10.3 | 2490 MB | **1.5454** | ✅ Aprovado (Campeão) |
| 7 | `Local - Bonsai 27B (GGUF)` | qwen35 | GGUF_CUSTOM | 3.54 GB | `ik_llama_vanguard` | 7450.8 | 1064.4 | 0.9 | 943 MB | **0.6903** | ✅ Aprovado (Campeão) |
| 8 | `Local - Bonsai 27B (GGUF)` | qwen35 | GGUF_CUSTOM | 3.54 GB | `ik_llama_vanguard` | 7799.1 | 1114.2 | 0.9 | 943 MB | **0.6595** | ✅ Aprovado (Campeão) |
### 🎯 Tier 4 (Speculative Drafters)

| # | Modelo | Família | Quant | Tamanho | Motor Campeão | TTFT (ms) | TPOT (ms) | TPS | VRAM Pico | Score E³ | Status |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| 1 | `Local - Bonsai 27B Dspark (GGUF)` | dspark | GGUF | 1.66 GB | `ik_llama_vanguard` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 2 | `Qwen - Qwen3 (GGUF)` | dflash | Q4_K_M | 363.8 MB | `ik_llama_vanguard` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |
| 3 | `Local - Laguna XS 2 (GGUF)` | dflash | Q4_K_M | 297.6 MB | `ik_llama_vanguard` | N/A | N/A | N/A | 0 MB | **0.0000** | ⏳ Pendente |

---

## 🧬 4. COLISEU COGNITIVO: MATRIZ DAS 4 TRILHAS QUALITATIVAS (TIER 2)

| Modelo | Tier | 🛠️ Tools (BFCL v4) | 🦀 Rust AST Code | 🧠 CoT Reasoning E³ | 👁️ VLM VQA | Veredito Cognitivo |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| `gliclass_multilang` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Transformers (GGUF)` | Tier 0 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Local - Smollm2 360M 8k Lc100K Mix1 Ep2 (GGUF)` | Tier 0 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Unsloth - Unsloth O97Brpro (GGUF)` | Tier 0 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Local - Neuralai Mamba K1 V3 Merged (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Mamba 790m Hf (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Stress Test Model (GGUF)` | Tier 0 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Qwen - Deepseek R1 Distill 1 (GGUF)` | Tier 0.5 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Local - Liquidai LFM2 5 1 (GGUF)` | Tier 0.5 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Qwen - Qwen2 (GGUF)` | Tier 1 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Unsloth - Unsloth Nbtyw0Rt (GGUF)` | Tier 1 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Local - Mamba Codestral 7B V0 (GGUF)` | Tier 1 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Local - SmolLM3 3B (GGUF)` | Tier 1 | 50% | 50% | 50% | N/A | Pronto para Roteamento |
| `Qwen - Qwen3 (GGUF)` | Tier 1 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Qwen - Parable V2 4B Merged (GGUF)` | Tier 1 | 50% | 50% | 50% | N/A | Pronto para Roteamento |
| `Local - Zamba2 2 (Q4_0)` | Tier 1 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `model` | Tier 1 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `LMStudio - Community Nn (GGUF)` | Tier 1 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Phi-4 Mini Instruct (GGUF)` | Tier 1 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Phi-4 Mini Reasoning (GGUF)` | Tier 1 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Falcon3 Mamba 7B Instruct (GGUF)` | Tier 1 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - 7B (GGUF)` | Tier 1 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Phi-4 Mini Instruct (GGUF)` | Tier 1 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Mamba Codestral 7B V0 (GGUF)` | Tier 1 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Safetensors (GGUF)` | Tier 2 | 50% | 50% | 50% | N/A | Pronto para Roteamento |
| `Local - Essentialai Rnj 1 Instruct (GGUF)` | Tier 2 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Gemma 4 E2B (GGUF)` | Tier 3 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Local - Gemma 4 E2B It Ultra Uncensored Heretic (GGUF)` | Tier 3 | 100% | 100% | 100% | N/A | Pronto para Roteamento |
| `Microsoft - Fara 7B (GGUF)` | Tier 3 | 50% | 50% | 50% | N/A | Pronto para Roteamento |
| `Local - Synthagent SFT UI TARS 1 (GGUF)` | Tier 3 | 50% | 50% | 50% | N/A | Pronto para Roteamento |
| `Microsoft - Fara1 (GGUF)` | Tier 3 | 50% | 50% | 50% | N/A | Pronto para Roteamento |
| `Nimbus Labs - 4B (GGUF)` | Tier 3 | 50% | 50% | 50% | N/A | Pronto para Roteamento |
| `Local - Bonsai 27B (GGUF)` | Tier 3 | 50% | 50% | 50% | N/A | Pronto para Roteamento |
| `Local - Bonsai 27B (GGUF)` | Tier 3 | 50% | 50% | 50% | N/A | Pronto para Roteamento |
| `Local - Bonsai 27B Dspark (GGUF)` | Tier 4 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Qwen - Qwen3 (GGUF)` | Tier 4 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |
| `Local - Laguna XS 2 (GGUF)` | Tier 4 | N/A | N/A | N/A | N/A | Aguardando Avaliação Qualitativa |

---

## ⚡ 5. ACELERAÇÃO ESPECULATIVA & MTP DRAFTING (TIER 4)

| Modelo Rascunho | Formato | Taxa de Aceitação Alpha (α) | Speedup Projetado | Veredito FinOps |
| :--- | :---: | :---: | :---: | :--- |
| `Local - Bonsai 27B Dspark (GGUF)` | GGUF Draft | N/A (Aguardando Tier 4) | 1.00x | ⏳ Pendente de Combate |
| `Qwen - Qwen3 (GGUF)` | GGUF Draft | N/A (Aguardando Tier 4) | 1.00x | ⏳ Pendente de Combate |
| `Local - Laguna XS 2 (GGUF)` | GGUF Draft | N/A (Aguardando Tier 4) | 1.00x | ⏳ Pendente de Combate |

---

## 🧩 6. PAREAMENTO MULTIMODAL & MÓDULOS AUXILIARES (11)

| # | Nome do Módulo | Tipo | Tamanho | Caminho Físico |
| :--- | :--- | :---: | :---: | :--- |
| 1 | `Bonsai-27B-mmproj-BF16.gguf` | `VISION_PROJECTOR` | 888.0 MB | `C:\Users\rosas\.lmstudio\models\AI-Joe-git\Bonsai-27B-gguf\Bonsai-27B-mmproj-BF16.gguf` |
| 2 | `Qwen3.5-4B-Uncensored-HauhauCS-Aggressive-MTP-Q4_K_M.gguf` | `MTP_ADAPTER` | 2.60 GB | `C:\Users\rosas\.lmstudio\models\AIOpsInSpace\Qwen3.5-4B-Uncensored-HauhauCS-Aggressive-MTP\Qwen3.5-4B-Uncensored-HauhauCS-Aggressive-MTP-Q4_K_M.gguf` |
| 3 | `mmproj-microsoft.Fara1.5-4B.f16.gguf` | `VISION_PROJECTOR` | 641.3 MB | `C:\Users\rosas\.lmstudio\models\DevQuasar\microsoft.Fara1.5-4B-GGUF\mmproj-microsoft.Fara1.5-4B.f16.gguf` |
| 4 | `DeepSeek-V4-Pro-Qwen3.5-4B-MTP-Q4_K_S.gguf` | `MTP_ADAPTER` | 2.45 GB | `C:\Users\rosas\.lmstudio\models\Jackrong\DeepSeek-V4-Pro-Qwen3.5-4B-MTP-GGUF\DeepSeek-V4-Pro-Qwen3.5-4B-MTP-Q4_K_S.gguf` |
| 5 | `mmproj-F32.gguf` | `VISION_PROJECTOR` | 1.24 GB | `C:\Users\rosas\.lmstudio\models\Jackrong\DeepSeek-V4-Pro-Qwen3.5-4B-MTP-GGUF\mmproj-F32.gguf` |
| 6 | `mmproj-Nimbus-4B-F16.gguf` | `VISION_PROJECTOR` | 641.3 MB | `C:\Users\rosas\.lmstudio\models\Nimbus-Labs\Nimbus-4B-GGUF\mmproj-Nimbus-4B-F16.gguf` |
| 7 | `mmproj-microsoft_Fara-7B-f16.gguf` | `VISION_PROJECTOR` | 1.26 GB | `C:\Users\rosas\.lmstudio\models\bartowski\microsoft_Fara-7B-GGUF\mmproj-microsoft_Fara-7B-f16.gguf` |
| 8 | `gemma-4-E2B-it-mmproj-BF16.gguf` | `VISION_PROJECTOR` | 941.1 MB | `C:\Users\rosas\.lmstudio\models\llmfan46\gemma-4-E2B-it-ultra-uncensored-heretic-GGUF\gemma-4-E2B-it-mmproj-BF16.gguf` |
| 9 | `mmproj-Bonsai-27B-BF16.gguf` | `VISION_PROJECTOR` | 888.0 MB | `C:\Users\rosas\.lmstudio\models\lmstudio-community\Bonsai-27B-GGUF\mmproj-Bonsai-27B-BF16.gguf` |
| 10 | `mmproj-gemma-4-E2B-it-BF16.gguf` | `VISION_PROJECTOR` | 941.1 MB | `C:\Users\rosas\.lmstudio\models\lmstudio-community\gemma-4-E2B-it-GGUF\mmproj-gemma-4-E2B-it-BF16.gguf` |
| 11 | `ggml-model-i2_s.gguf` | `SPECIALIZED_QUANT` | 1.11 GB | `C:\Users\rosas\.lmstudio\models\microsoft\bitnet-b1.58-2B-4T-gguf\ggml-model-i2_s.gguf` |

---

## 📦 7. MODELOS EMBARCADOS & CORE INTERNOS (`src-tauri/models`) (5)

| # | Nome do Arquivo | Categoria | Formato | Tamanho | Descrição |
| :--- | :--- | :---: | :---: | :---: | :--- |
| 1 | `gliclass_multilang.onnx.data` | `MODEL_WEIGHTS` | `DATA` | 3.18 GB | Modelo ONNX de Classificação de Intenções / NER |
| 2 | `tokenizer.json` | `TOKENIZER_CONFIG` | `JSON` | 15.6 MB | Arquivo de Configuração / Tokenizer Core |
| 3 | `gliclass_multilang.onnx` | `MODEL_WEIGHTS` | `ONNX` | 4.0 MB | Modelo ONNX de Classificação de Intenções / NER |
| 4 | `tokenizer_recovered.json` | `TOKENIZER_CONFIG` | `JSON` | 0.0 MB | Arquivo de Configuração / Tokenizer Core |
| 5 | `tokenizer_config.json` | `TOKENIZER_CONFIG` | `JSON` | 0.0 MB | Arquivo de Configuração / Tokenizer Core |

---

## 🛡️ 8. DISJUNTORES DE SAÚDE & QUARENTENAS TÉRMICAS (CIRCUIT BREAKERS)

✅ **Nenhum modelo em quarentena.** Todos os modelos ativos operam dentro da barreira térmica e de estabilidade.

---

## 📝 9. FICHA TÉCNICA E DOSSIÊ INDIVIDUAL DOS MODELOS

### 1. `gliclass_multilang`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `ort_scorer`
- **Caminho Físico:** `Z:\souls_mc\src-tauri\models\GLiClass_Multilang_Ultra\gliclass_multilang.onnx`
- **Metadados:** Família `gliclass` | Parâmetros `300M` | Contexto Máximo `2048` tokens | Quant `ONNX-F16`
- **Desempenho de Silício:** TTFT `48.93 ms` | TPOT `4.45 ms` | Throughput `224.7 tok/s` | VRAM Pico `40 MB` | **Score E³ `607.6446`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `607.6446` despachado pelo `ort_scorer`.

### 2. `Local - Transformers (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Ma7ee7\SmolLM2-135M-Reasoning-5K-GGUF\SmolLM2-135M-Reasoning-5K-Q4_K_M.gguf`
- **Metadados:** Família `llama` | Parâmetros `Unknown` | Contexto Máximo `8192` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `954.63 ms` | TPOT `12.89 ms` | Throughput `77.6 tok/s` | VRAM Pico `0 MB` | **Score E³ `581.3801`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `581.3801` despachado pelo `ik_llama_vanguard`.

### 3. `Local - Smollm2 360M 8k Lc100K Mix1 Ep2 (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\HuggingFaceTB\SmolLM2-360M-Instruct-GGUF\smollm2-360m-instruct-q8_0.gguf`
- **Metadados:** Família `llama` | Parâmetros `Unknown` | Contexto Máximo `8192` tokens | Quant `Q8_0`
- **Desempenho de Silício:** TTFT `579.11 ms` | TPOT `14.15 ms` | Throughput `70.7 tok/s` | VRAM Pico `0 MB` | **Score E³ `550.5181`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `550.5181` despachado pelo `ik_llama_vanguard`.

### 4. `Unsloth - Unsloth O97Brpro (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\hypaai\Hypa-SmolLM-135M-Instruct-GGUF\smollm-135m-instruct.Q4_K_M.gguf`
- **Metadados:** Família `llama` | Parâmetros `Unknown` | Contexto Máximo `2048` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `153.6 ms` | TPOT `14.53 ms` | Throughput `68.8 tok/s` | VRAM Pico `0 MB` | **Score E³ `529.316`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `529.316` despachado pelo `ik_llama_vanguard`.

### 5. `Local - Neuralai Mamba K1 V3 Merged (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `pulp_lele`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Subject-Emu-5259\NeuralAI-Mamba-K1\NeuralAI-Mamba-K1-v3.Q4_K_M.gguf`
- **Metadados:** Família `mamba` | Parâmetros `Unknown` | Contexto Máximo `1048576` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 6. `Local - Mamba 790m Hf (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `pulp_lele`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Subject-Emu-5259\NeuralAI-Mamba-K2\mamba-790m-hf.Q4_K_M.gguf`
- **Metadados:** Família `mamba` | Parâmetros `Unknown` | Contexto Máximo `1048576` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 7. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmp21Zk0L\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 8. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmp4OBef7\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 9. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpAFSxx0\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 10. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpBVDKLG\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 11. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpCB6RRb\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 12. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpG1LDH7\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 13. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpH8hWWq\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 14. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpHSpijG\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 15. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpHlLDkj\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 16. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpK5N97y\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 17. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpLYM05l\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 18. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpN9CLVt\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 19. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpNNjHQS\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 20. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpNjYKgy\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 21. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpOqOmxk\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 22. `Local - Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpPmj5m0\test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 23. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpRqEinQ\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 24. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpUUPPll\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 25. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpVaOuNu\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 26. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpVxzu4V\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 27. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpWpBZNv\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 28. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpXANvxM\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 29. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpXFt2nc\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 30. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpYyGTkP\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 31. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpa9IsrS\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 32. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpaHeuxU\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 33. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpejMGrG\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 34. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpgdQhZI\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 35. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpi6E6Zx\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 36. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpiZBXuK\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 37. `Local - Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpj4vjtQ\test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 38. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpjWYfpJ\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 39. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpkIUTgw\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 40. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmporff6Y\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 41. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpqxjlzs\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 42. `Local - Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpscpGIM\test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 43. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpw224Fw\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 44. `Local - Stress Test Model (GGUF)`
- **Tier Operacional:** `Tier 0 (Bootstrap & CPU Sanity)` | **Motor Campeão:** `llama_cpp`
- **Caminho Físico:** `C:\Users\rosas\AppData\Local\Temp\.tmpwtMa61\stress_test_model.gguf`
- **Metadados:** Família `Generic` | Parâmetros `Unknown` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 45. `Qwen - Deepseek R1 Distill 1 (GGUF)`
- **Tier Operacional:** `Tier 0.5 (Sensor Epistêmico)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\DeepSeek-R1-Distill-Qwen-1.5B-GGUF\DeepSeek-R1-Distill-Qwen-1.5B-Q8_0.gguf`
- **Metadados:** Família `qwen2` | Parâmetros `1.5B` | Contexto Máximo `131072` tokens | Quant `Q8_0`
- **Desempenho de Silício:** TTFT `1009.97 ms` | TPOT `11.84 ms` | Throughput `84.5 tok/s` | VRAM Pico `0 MB` | **Score E³ `633.7418`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `633.7418` despachado pelo `ik_llama_vanguard`.

### 46. `Local - Liquidai LFM2 5 1 (GGUF)`
- **Tier Operacional:** `Tier 0.5 (Sensor Epistêmico)` | **Motor Campeão:** `llama_upstream`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\LFM2.5-1.2B-Instruct-GGUF\LFM2.5-1.2B-Instruct-Q8_0.gguf`
- **Metadados:** Família `lfm2` | Parâmetros `1.2B` | Contexto Máximo `128000` tokens | Quant `Q8_0`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 47. `Qwen - Qwen2 (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\katanemo\Arch-Function-Chat-3B.gguf\Arch-Function-Chat-3B-Q5_K_M.gguf`
- **Metadados:** Família `qwen2` | Parâmetros `3B` | Contexto Máximo `32768` tokens | Quant `Q5_K_M`
- **Desempenho de Silício:** TTFT `734.63 ms` | TPOT `30.8 ms` | Throughput `32.5 tok/s` | VRAM Pico `2052 MB` | **Score E³ `15.2879`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `15.2879` despachado pelo `ik_llama_vanguard`.

### 48. `Unsloth - Unsloth Nbtyw0Rt (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\stevenlearns\qwen3.5-4B-super-coder\qwen3.5-4B-super-coder.Q4_0.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `4B` | Contexto Máximo `262144` tokens | Quant `Q4_0`
- **Desempenho de Silício:** TTFT `3582.21 ms` | TPOT `41.98 ms` | Throughput `23.8 tok/s` | VRAM Pico `2402 MB` | **Score E³ `7.3045`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `7.3045` despachado pelo `ik_llama_vanguard`.

### 49. `Local - Mamba Codestral 7B V0 (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `pulp_lele`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Agnuxo\Mamba-Codestral-7B-Instruct_CODE_Python-Spanish_English_GGUF_4bit\unsloth.Q4_K_M.gguf`
- **Metadados:** Família `llama` | Parâmetros `7B` | Contexto Máximo `32768` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `2591.21 ms` | TPOT `30.36 ms` | Throughput `32.9 tok/s` | VRAM Pico `3998 MB` | **Score E³ `6.1687`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `6.1687` despachado pelo `pulp_lele`.

### 50. `Local - SmolLM3 3B (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\bartowski\HuggingFaceTB_SmolLM3-3B-GGUF\HuggingFaceTB_SmolLM3-3B-Q5_K_M.gguf`
- **Metadados:** Família `smollm3` | Parâmetros `3B` | Contexto Máximo `65536` tokens | Quant `Q5_K_M`
- **Desempenho de Silício:** TTFT `1805.64 ms` | TPOT `71.01 ms` | Throughput `14.1 tok/s` | VRAM Pico `2042 MB` | **Score E³ `3.8684`**
- **Avaliação Qualitativa:** Tools BFCL: `50%` | Rust AST: `50%` | Reasoning CoT: `50%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `3.8684` despachado pelo `ik_llama_vanguard`.

### 51. `Qwen - Qwen3 (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\armandosds\qwen3.5-4b-agentic-coder-v4-i1-GGUF\qwen3.5-4b-agentic-coder-v4.i1-Q4_K_S.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `4B` | Contexto Máximo `262144` tokens | Quant `Q4_K_S`
- **Desempenho de Silício:** TTFT `1672.33 ms` | TPOT `88.45 ms` | Throughput `11.3 tok/s` | VRAM Pico `2420 MB` | **Score E³ `3.6409`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `3.6409` despachado pelo `ik_llama_vanguard`.

### 52. `Qwen - Parable V2 4B Merged (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AnkitAI\Parable-Qwen3-4B-Claude-Fable-5-GGUF\Parable-Qwen3-4B-Claude-Fable-5-GGUF-Q4_K_M.gguf`
- **Metadados:** Família `qwen3` | Parâmetros `4B` | Contexto Máximo `40960` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `1267.26 ms` | TPOT `81.7 ms` | Throughput `12.2 tok/s` | VRAM Pico `2299 MB` | **Score E³ `2.0194`**
- **Avaliação Qualitativa:** Tools BFCL: `50%` | Rust AST: `50%` | Reasoning CoT: `50%`
- **Módulos Anexados:** `Qwen3.5-4B-Uncensored-HauhauCS-Aggressive-MTP-Q4_K_M.gguf` (2.60 GB), `DeepSeek-V4-Pro-Qwen3.5-4B-MTP-Q4_K_S.gguf` (2.45 GB)
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `2.0194` despachado pelo `ik_llama_vanguard`.

### 53. `Local - Zamba2 2 (Q4_0)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `pulp_lele`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\EchoLabs33\Zamba2-2.7B-Instruct-v2-GGUF\zamba2-2.7b-instruct-v2-q4_0.gguf`
- **Metadados:** Família `zamba2` | Parâmetros `2.7B` | Contexto Máximo `4096` tokens | Quant `Q4_0`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `2055 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 54. `model`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `mistral_rs`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\knowledgator\gliclass-multilang-ultra\model.safetensors`
- **Metadados:** Família `safetensors` | Parâmetros `unknown` | Contexto Máximo `4096` tokens | Quant `F16`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `3131 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 55. `LMStudio - Community Nn (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `llama_upstream`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\NVIDIA-Nemotron-3-Nano-4B-GGUF\NVIDIA-Nemotron-3-Nano-4B-Q4_K_M.gguf`
- **Metadados:** Família `nemotron_h` | Parâmetros `4B` | Contexto Máximo `1048576` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `2606 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 56. `Local - Phi-4 Mini Instruct (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `llama_upstream`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\Phi-4-mini-instruct-GGUF\Phi-4-mini-instruct-Q4_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `2294 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 57. `Local - Phi-4 Mini Reasoning (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `llama_upstream`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\Phi-4-mini-reasoning-GGUF\Phi-4-mini-reasoning-Q4_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `2294 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 58. `Local - Falcon3 Mamba 7B Instruct (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `pulp_lele`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\mradermacher\Falcon3-Mamba-7B-Instruct-i1-GGUF\Falcon3-Mamba-7B-Instruct.i1-IQ3_M.gguf`
- **Metadados:** Família `mamba` | Parâmetros `7B` | Contexto Máximo `1048576` tokens | Quant `GGUF_CUSTOM`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 59. `Local - 7B (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `llama_upstream`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\tencent\Hy-MT2-7B-GGUF\Hy-MT2-7B-Q4_K_M.gguf`
- **Metadados:** Família `hunyuan-dense` | Parâmetros `7B` | Contexto Máximo `262144` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `4171 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 60. `Local - Phi-4 Mini Instruct (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `llama_upstream`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\unsloth\Phi-4-mini-instruct-GGUF\Phi-4-mini-instruct-Q5_K_M.gguf`
- **Metadados:** Família `phi3` | Parâmetros `Unknown` | Contexto Máximo `131072` tokens | Quant `Q5_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `2616 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 61. `Local - Mamba Codestral 7B V0 (GGUF)`
- **Tier Operacional:** `Tier 1 (Live Chat & Master)` | **Motor Campeão:** `pulp_lele`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\viniciusianni\Mamba-Codestral-7B-v0.1-Q4_K_M-GGUF\mamba-codestral-7b-v0.1-q4_k_m.gguf`
- **Metadados:** Família `mamba2` | Parâmetros `7B` | Contexto Máximo `1048576` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `3794 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 62. `Local - Safetensors (GGUF)`
- **Tier Operacional:** `Tier 2 (Background Agent & MoE Híbrido)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\mudler\Laguna-XS-2.1-APEX-GGUF\Laguna-XS-2.1-APEX-I-Compact.gguf`
- **Metadados:** Família `laguna` | Parâmetros `Unknown` | Contexto Máximo `262144` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `5832.72 ms` | TPOT `833.25 ms` | Throughput `1.2 tok/s` | VRAM Pico `4173 MB` | **Score E³ `0.2156`**
- **Avaliação Qualitativa:** Tools BFCL: `50%` | Rust AST: `50%` | Reasoning CoT: `50%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `0.2156` despachado pelo `ik_llama_vanguard`.

### 63. `Local - Essentialai Rnj 1 Instruct (GGUF)`
- **Tier Operacional:** `Tier 2 (Background Agent & MoE Híbrido)` | **Motor Campeão:** `llama_upstream`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\rnj-1-instruct-GGUF\rnj-1-instruct-Q4_K_M.gguf`
- **Metadados:** Família `gemma3` | Parâmetros `Unknown` | Contexto Máximo `32768` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `4151 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 64. `Local - Gemma 4 E2B (GGUF)`
- **Tier Operacional:** `Tier 3 (Vision & Multimodal VLM)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\gemma-4-E2B-it-GGUF\gemma-4-E2B-it-Q4_K_M.gguf`
- **Metadados:** Família `gemma4` | Parâmetros `Unknown` | Contexto Máximo `131072` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `1100.34 ms` | TPOT `25.88 ms` | Throughput `38.6 tok/s` | VRAM Pico `3142 MB` | **Score E³ `9.7903`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** `mmproj-gemma-4-E2B-it-BF16.gguf` (Pareado SQLite)
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `9.7903` despachado pelo `ik_llama_vanguard`.

### 65. `Local - Gemma 4 E2B It Ultra Uncensored Heretic (GGUF)`
- **Tier Operacional:** `Tier 3 (Vision & Multimodal VLM)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\llmfan46\gemma-4-E2B-it-ultra-uncensored-heretic-GGUF\gemma-4-E2B-it-ultra-uncensored-heretic-Q4_K_M.gguf`
- **Metadados:** Família `gemma4` | Parâmetros `Unknown` | Contexto Máximo `131072` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `1007.5 ms` | TPOT `27.52 ms` | Throughput `36.3 tok/s` | VRAM Pico `3142 MB` | **Score E³ `9.6354`**
- **Avaliação Qualitativa:** Tools BFCL: `100%` | Rust AST: `100%` | Reasoning CoT: `100%`
- **Módulos Anexados:** `gemma-4-E2B-it-mmproj-BF16.gguf` (Pareado SQLite)
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `9.6354` despachado pelo `ik_llama_vanguard`.

### 66. `Microsoft - Fara 7B (GGUF)`
- **Tier Operacional:** `Tier 3 (Vision & Multimodal VLM)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\bartowski\microsoft_Fara-7B-GGUF\microsoft_Fara-7B-IQ3_M.gguf`
- **Metadados:** Família `qwen2vl` | Parâmetros `7B` | Contexto Máximo `128000` tokens | Quant `GGUF_CUSTOM`
- **Desempenho de Silício:** TTFT `2559.26 ms` | TPOT `71.36 ms` | Throughput `14.0 tok/s` | VRAM Pico `3274 MB` | **Score E³ `2.2145`**
- **Avaliação Qualitativa:** Tools BFCL: `50%` | Rust AST: `50%` | Reasoning CoT: `50%`
- **Módulos Anexados:** `mmproj-microsoft_Fara-7B-f16.gguf` (Pareado SQLite)
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `2.2145` despachado pelo `ik_llama_vanguard`.

### 67. `Local - Synthagent SFT UI TARS 1 (GGUF)`
- **Tier Operacional:** `Tier 3 (Vision & Multimodal VLM)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\mradermacher\SynthAgent-SFT-UI-TARS-1.5-7B-i1-GGUF\SynthAgent-SFT-UI-TARS-1.5-7B.i1-IQ3_M.gguf`
- **Metadados:** Família `qwen2vl` | Parâmetros `7B` | Contexto Máximo `128000` tokens | Quant `GGUF_CUSTOM`
- **Desempenho de Silício:** TTFT `2312.15 ms` | TPOT `54.71 ms` | Throughput `18.3 tok/s` | VRAM Pico `3274 MB` | **Score E³ `2.1889`**
- **Avaliação Qualitativa:** Tools BFCL: `50%` | Rust AST: `50%` | Reasoning CoT: `50%`
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `2.1889` despachado pelo `ik_llama_vanguard`.

### 68. `Microsoft - Fara1 (GGUF)`
- **Tier Operacional:** `Tier 3 (Vision & Multimodal VLM)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\DevQuasar\microsoft.Fara1.5-4B-GGUF\microsoft.Fara1.5-4B.f16.gguf.Q4_K_M.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `4B` | Contexto Máximo `262144` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `1878.19 ms` | TPOT `85.44 ms` | Throughput `11.7 tok/s` | VRAM Pico `2490 MB` | **Score E³ `2.0768`**
- **Avaliação Qualitativa:** Tools BFCL: `50%` | Rust AST: `50%` | Reasoning CoT: `50%`
- **Módulos Anexados:** `mmproj-microsoft.Fara1.5-4B.f16.gguf` (Pareado SQLite)
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `2.0768` despachado pelo `ik_llama_vanguard`.

### 69. `Nimbus Labs - 4B (GGUF)`
- **Tier Operacional:** `Tier 3 (Vision & Multimodal VLM)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Nimbus-Labs\Nimbus-4B-GGUF\Nimbus-4B-Q4_K_M.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `4B` | Contexto Máximo `262144` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `1469.66 ms` | TPOT `97.13 ms` | Throughput `10.3 tok/s` | VRAM Pico `2490 MB` | **Score E³ `1.5454`**
- **Avaliação Qualitativa:** Tools BFCL: `50%` | Rust AST: `50%` | Reasoning CoT: `50%`
- **Módulos Anexados:** `mmproj-Nimbus-4B-F16.gguf` (Pareado SQLite)
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `1.5454` despachado pelo `ik_llama_vanguard`.

### 70. `Local - Bonsai 27B (GGUF)`
- **Tier Operacional:** `Tier 3 (Vision & Multimodal VLM)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AI-Joe-git\Bonsai-27B-gguf\Bonsai-27B-Q1_0.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `27B` | Contexto Máximo `262144` tokens | Quant `GGUF_CUSTOM`
- **Desempenho de Silício:** TTFT `7450.83 ms` | TPOT `1064.4 ms` | Throughput `0.9 tok/s` | VRAM Pico `943 MB` | **Score E³ `0.6903`**
- **Avaliação Qualitativa:** Tools BFCL: `50%` | Rust AST: `50%` | Reasoning CoT: `50%`
- **Módulos Anexados:** `Bonsai-27B-mmproj-BF16.gguf` (Pareado SQLite)
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `0.6903` despachado pelo `ik_llama_vanguard`.

### 71. `Local - Bonsai 27B (GGUF)`
- **Tier Operacional:** `Tier 3 (Vision & Multimodal VLM)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\lmstudio-community\Bonsai-27B-GGUF\Bonsai-27B-Q1_0.gguf`
- **Metadados:** Família `qwen35` | Parâmetros `27B` | Contexto Máximo `262144` tokens | Quant `GGUF_CUSTOM`
- **Desempenho de Silício:** TTFT `7799.15 ms` | TPOT `1114.16 ms` | Throughput `0.9 tok/s` | VRAM Pico `943 MB` | **Score E³ `0.6595`**
- **Avaliação Qualitativa:** Tools BFCL: `50%` | Rust AST: `50%` | Reasoning CoT: `50%`
- **Módulos Anexados:** `mmproj-Bonsai-27B-BF16.gguf` (Pareado SQLite)
- **Veredito ParetoBandit:** 🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `0.6595` despachado pelo `ik_llama_vanguard`.

### 72. `Local - Bonsai 27B Dspark (GGUF)`
- **Tier Operacional:** `Tier 4 (Speculative Drafters)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\AI-Joe-git\Bonsai-27B-gguf\Bonsai-27B-dspark-Q4_1.gguf`
- **Metadados:** Família `dspark` | Parâmetros `27B` | Contexto Máximo `4096` tokens | Quant `GGUF`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** `Bonsai-27B-mmproj-BF16.gguf` (Pareado SQLite)
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 73. `Qwen - Qwen3 (GGUF)`
- **Tier Operacional:** `Tier 4 (Speculative Drafters)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\Anbeeld\Qwen3.5-4B-DFlash-GGUF\qwen35-4b-dflash-Q4_K_M.gguf`
- **Metadados:** Família `dflash` | Parâmetros `4B` | Contexto Máximo `262144` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

### 74. `Local - Laguna XS 2 (GGUF)`
- **Tier Operacional:** `Tier 4 (Speculative Drafters)` | **Motor Campeão:** `ik_llama_vanguard`
- **Caminho Físico:** `C:\Users\rosas\.lmstudio\models\RespectMathias\Laguna-XS-2.1-DSpark-GGUF\Laguna-XS-2.1-DSpark-Q4_K_M.gguf`
- **Metadados:** Família `dflash` | Parâmetros `Unknown` | Contexto Máximo `262144` tokens | Quant `Q4_K_M`
- **Desempenho de Silício:** TTFT `0.0 ms` | TPOT `0.0 ms` | Throughput `0.0 tok/s` | VRAM Pico `0 MB` | **Score E³ `0.0`**
- **Módulos Anexados:** Nenhum
- **Veredito ParetoBandit:** 🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark.

---
*Fim do Dossiê de Inventário SOULS V5. Gerado automaticamente via `souls_llms_inventory_viewer.py`.*