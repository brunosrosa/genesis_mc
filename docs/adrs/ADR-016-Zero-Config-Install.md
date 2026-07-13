---
id: "ADR-016"
title: "ADR-016-Zero-Config-Install"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Garante a distribuição auto-suficiente do SODA empacotado pelo Tauri com zero dependências externas no host."
---

# ADR-016-Zero-Config-Install

## Status
Aceito (Ativo e Inegociável)

## Contexto
O SODA foi arquitetado para ser uma solução soberana "Local-First". Contudo, sistemas operacionais de usuários divergem massivamente em termos de hardware físico (CPUs Intel/AMD, quantidade de RAM e presença/capacidade de dGPUs dedicadas). Exigir que usuários finais realizem configurações manuais complexas de drivers, alocação de threads e seleção de formatos de quantização de tensores inviabiliza a facilidade de uso, provocando falhas graves de Out-of-Memory (OOM) na GPU e sobredimensionamento letal de threads.

## Decisão
Implementar um motor nativo de **Auto-Profiling de Hardware** em Rust, executado compulsoriamente na inicialização do sistema (Boot) e em transições de energia:
1. **Mapeamento Mecânico:** O core Rust faz chamadas de baixo nível ao sistema operacional para mapear:
   - Quantidade de memória RAM livre e total.
   - Presença de dGPUs e VRAM dedicada via APIs de baixo nível (CUDA/WGPU).
   - Topologia de núcleos de CPU (Cores de Performance vs Eficiência) e suporte a extensões de vetorização (AVX2, AVX-512, NEON).
   - Banda e vias físicas do **PCIe** via `all-smi` e biblioteca **pcics**.
2. **Calibração Termodinâmica Dinâmica:** A partir do profiling físico, o motor ajusta dinamicamente os parâmetros em tempo de execução:
   - **Tamanho do Bloco de Threads Tokio:** Alocação exata de trabalhadores assíncronos.
   - **Limites Físicos da MoE Local:** Se a VRAM for <6GB, o sistema bloqueia o carregamento automático de modelos especialistas 7B+, limitando-se ao Gateway Cognitivo local na CPU e acionando o Fallback FinOps via nuvem de forma invisível.
   - **Modo do Barramento PCIe:** Ativação ou supressão da Unified Memory da GPU (`GGML_CUDA_ENABLE_UNIFIED_MEMORY`).
   - **Lei dos 32 GB/s:** Se a banda efetiva do PCIe for detectada como inferior a **32 GB/s**, o orquestrador proíbe sumariamente o offloading dinâmico do KV Cache para a RAM do host e força o corte de contexto para evitar engasgos catastróficos.
3. **Instalação Sem Estado:** Toda a configuração de hardware é puramente efêmera e recalculada no boot, prevenindo que alterações físicas na máquina do usuário (ex: adição de RAM ou dGPU) quebrem o software.

## Consequências
- **Instalação Instantânea:** Experiência "Zero-Config" verdadeira. O usuário apenas executa o binário do Tauri e o SODA otimiza-se sozinho ao ambiente hospedeiro.
- **Robustez Térmica:** O software calibra sua pegada energética com base no estado térmico e na capacidade de resfriamento físico do host, prevenindo *thermal throttling*.
- **Manutenibilidade:** Eliminação total de arquivos de configuração estática de hardware difíceis de depurar.

## Restrições Bare-Metal
- **Tempo Máximo de Boot Profiling:** O mapeamento físico completo deve rodar em no máximo **300ms** na inicialização.
- **Reservas Rígidas:** No profiling, o sistema deve compulsoriamente reservar no mínimo **4GB de RAM** para o sistema operacional hospedeiro e **1.5GB de VRAM** para evitar o congelamento de processos gráficos essenciais da dGPU.
- **Profiling PCIe (pcics):** A leitura de banda e vias físicas do PCIe deve usar a biblioteca **pcics** (além do `all-smi`) durante o boot.
- **Lei dos 32 GB/s:** Se a banda detectada for $< 32$ GB/s, fica proibido o offloading dinâmico do KV Cache para a RAM do host; o orquestrador deve aplicar corte de contexto compulsório.
