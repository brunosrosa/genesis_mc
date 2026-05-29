# ADR-001-Core-Stack-Restrita

## Status
Aceito (Ativo e Inegociável)

## Contexto
O ecossistema **Genesis Mission Control (SODA)** opera sob restrições físicas de hardware rígidas no ambiente do usuário (RTX 2060m com 6GB VRAM, Intel i9 CPU e 32GB RAM). Runtimes tradicionais de desktop como Electron e frameworks baseados em Node.js geram grande sobrecarga computacional ("Daemon Bloat"), disputando barramento PCIe, CPU e memória física direta com motores de IA locais. Adicionalmente, interpretadores com Garbage Collection (GC) provocam latências estocásticas imprevisíveis e Event Loop Starvation, o que é inaceitável para uma Prótese de Função Executiva projetada para mentes neurodivergentes (2e/TDAH) que exige feedback mecânico e instantâneo.

## Decisão
Fica formalmente decidido que a arquitetura de produção do SODA é estritamente Bare-Metal e restrita às seguintes tecnologias:
1. **Backend & Core:** Desenvolvidos inteiramente em **Rust assíncrono (Tokio)**. Nenhuma lógica persistente em Node.js ou Python habitará a produção.
2. **Frontend & Interface:** Svelte 5 (Runes) + TypeScript empacotados via **Tauri v2**.
3. **Repúdio Absoluto:** Fica proibida a injeção ou persistência de daemons baseados em Electron, microsserviços Node.js locais ou interpretadores JS/Python pesados no pacote do produto. Qualquer lógica de terceiros que necessite de outras linguagens deve rodar como sidecar efêmero isolado em sandbox nativa e ser encerrada compulsoriamente via sinal atômico `SIGKILL` após o término da tarefa.

## Consequências
- **Ganhos de Performance:** Redução da pegada de RAM do shell gráfico de ~500MB (padrão Electron) para <40MB com Tauri v2.
- **Previsibilidade:** Eliminação de pausas de Garbage Collection no core do sistema, permitindo que a CPU i9 atue estritamente com latências determinísticas.
- **Complexidade de Engenharia:** Todo acoplamento e orquestração de sistema operacional exigem a escrita de bindings nativos em Rust (Tauri Commands), extinguindo a prática de scripts ad-hoc no frontend.

## Restrições Bare-Metal
- **Latência de Comunicação (IPC):** Chamadas de comandos nativos do Tauri devem responder em menos de **10ms**.
- **Consumo de Memória Gráfica:** A interface gráfica não pode ultrapassar **150MB de alocação de RAM** em repouso e deve rodar com renderização de baixo custo (LowPower API) na iGPU (Intel UHD 630), liberando a RTX 2060m dGPU exclusivamente para tensores de IA.
