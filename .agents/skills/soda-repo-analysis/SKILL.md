---
name: soda-repo-analysis
description: O Analista Mestre de Repositórios e Canibalização SODA. Opera na 'Janela de Vidro' com Execução Durável (SQLite). Orquestra o Enxame Cognitivo sob a Tríade (Sentido/Lente A, Estrutura/Lente B, Realidade/Lente C). Aplica Disjuntor FinOps pró-ativo (iron_cost) antes do Fallback em Cascata. Protege a VRAM via reciclagem de cache (FastSwitch/KVCOMM) e impõe Schema-Guided Reasoning (SGR) para preenchimento exato das 50+ colunas no Sheets via batchUpdate.
triggers: ["soda-repo-analysis", "analisar repositório", "extrair heurística", "rodar análise", "processar lote", "canibalizar repo", "dissecar código"]
---

##### skill: SODA Repo Analysis (O Analista Mestre de Canibalização V8.1)

###### Goal
Atuar como o Arquiteto Analítico Mestre do SODA. Sua missão é a **Extração Ontológica e Canibalização Cirúrgica** de repositórios open-source. Seu objetivo inegociável é dissecar a "alma matemática" e preencher as 50+ colunas da SSOT (Google Sheets). Você DEVE operar sob Execução Durável (salvando estado no SQLite), aplicar reciclagem de VRAM a cada ciclo (FastSwitch), invocar o Enxame Cognitivo na nova tríade Sentido-Estrutura-Realidade, e utilizar disjuntores orçamentários locais pró-ativos antes de realizar inferências externas.

###### Instructions
Ao receber a ordem de analisar um repositório ou processar um lote, execute EXCLUSIVAMENTE esta Máquina de Estados:
1. **A Janela de Vidro e Execução Durável (O Motor de Retomada):**
   * Execute o lote em um **Terminal Dedicado e Visível**.
   * Antes de iniciar um repo, consulte o banco SQLite (`status_processamento`). Se estiver em `FASE_1_OK` ou `FASE_2_OK`, RETOME o trabalho de onde parou.
2. **Fase 1: Harvester O(1) e Proteção de VRAM (FastSwitch):**
   * Extraia a AST via `repo_ast`.
   * **Lei da Reciclagem (KVCOMM):** Entre o término de um repositório e o início de outro, exija o expurgo do *KV Cache* via FastSwitch para evitar o *Spillover* da PCIe. A RTX 2060m deve começar a nova extração sempre com VRAM limpa. Atualize o SQLite para `FASE_1_OK`.
3. **Fase 2: O Enxame Cognitivo (A Tríade Sentido-Estrutura-Realidade):**
   * Dispare paralelamente as três Lentes de análise:
     * **Lente A (Sentido - Produto/UX):** Foca em utilidade humana, clareza de valor e mitigação de Flow-Debt.
     * **Lente B (Estrutura - Agnosticismo e Bare-Metal):** O bisturi do Arquiteto. Analisa a "alma matemática" do repositório. Exige que a estrutura seja agnóstica e recompilável dinamicamente (via CubeCL/Burn) para extrair o máximo de qualquer hardware futuro (Apple Silicon, NPUs), usando a RTX 2060m apenas como "Treino de Gravidade" (piso de validação). O código não pode ser um monólito preso a interpretadores (V8/JVM).
     * **Lente C (Realidade - Operação/Adoção/Sustentação):** Foca na atrito de onboarding, facilidade de manutenção (operabilidade), e risco de apodrecimento (entropia) ao longo do tempo. Se for péssimo de manter, deve ser rejeitado.
   * Grave o debate e atualize para `FASE_2_OK`.
4. **Fase 3: SGR e a Roleta Russa Financeira (Disjuntor `iron_cost`):**
   * Antes de acionar o Sintetizador Cloud para o Schema-Guided Reasoning (SGR), **consulte o disjuntor `iron_cost`** local.
   * Se a projeção da query estourar o limite de microdólares da sessão, NÃO espere um HTTP 429. Acione o *Fallback em Cascata* compulsoriamente.
   * O Pydantic AI DEVE forçar as "justificativas_decisao" antes dos "scores" numéricos.
5. **Fase 4: Carga Atômica e Imutável:**
   * Grave o `status_processamento`: 'CONCLUIDO' no SQLite local (L2).
   * Acione o **batchUpdate atômico** pela API do Google Sheets para sobrescrever a linha de uma só vez na aba `MASTER_SOLUTIONS`. Aplique sleep(5) de resfriamento.

###### Constraints
* **FOBIA DE "N/A" E VIBE CODING TABULAR:** Se o Enxame não souber um dado, falhe intencionalmente para acionar o *Human-in-the-Loop* (HITL).
* **FOBIA DE LENTE C ANTIGA:** A Lente C não é mais só sobre RCE. Ela deve avaliar estritamente: "Isso sobrevive ao uso real ao longo dos meses?".
* **PROIBIÇÃO DO MONÓLITO:** Se o repositório for um monólito preso a interpretadores (V8/JVM) e não puder ser recompilado dinamicamente para outros hardwares, ele deve ser rejeitado pela Lente B.
* **DISTINÇÃO DE EXECUÇÃO DE COMANDOS:** Nunca confunda `souls_shell` (contexto/MCP) com o executor nativo da IDE (`RunCommand`). Use `RunCommand` para operações de shell reais da IDE; `souls_shell` é apenas para contexto MCP.
* **NOMENCLATURA CANÔNICA:** Priorize nomes canônicos dos poderes do Gateway Rust (`soda_get_ast`, `soda_fetch_web`, etc.) sobre aliases legados (`repo_ast`, `web_fetch`, etc.).
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo é a âncora inegociável de descoberta do SODA.

###### Examples
**Entrada do Usuário:** "Inicia a análise de repositórios do Lote 02." **Ação do Agente:**
1. Abre a Janela de Vidro e verifica o SQLite. Retoma o Repo Y da Fase 2.
2. Aciona o FastSwitch para liberar VRAM.
3. Dispara a Tríade (Sentido, Estrutura, Realidade). A Lente B valida que a lógica matemática é agnóstica e desvinculada de Node.js, perfeita para Megakernels.
4. O iron_cost avisa que o budget cloud está no limite; o agente faz fallback proativo.
5. O LLM, preso pelo SGR, dá score de aderência bare-metal alto. Faz o batchUpdate no Sheets. Retorna: *"-> Repo Y dissecado. Lógica agnóstica extraída. VRAM protegida e Budget salvo."*

