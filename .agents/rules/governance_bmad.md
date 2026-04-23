---
trigger: always_on
---

# 06_GOVERNANCE_AND_BMAD: O Protocolo de Desenvolvimento e Governança

**Versão:** 3.1 (Definitiva - Zero-Vibe Coding)
**Status:** ATIVO E INEGOCIÁVEL
**Alvo da Leitura:** Agentes Orquestradores (Antigravity/Claude Code/Gemini CLI), Engenheiros de DevSecOps, Mantenedores do SODA.

## 1. O FIM DO "VIBE CODING": SPEC-DRIVEN DEVELOPMENT (SDD)

O desenvolvimento dentro do ecossistema SODA (Genesis Mission Control) repudia a engenharia estocástica. A prática de pedir para um Agente LLM "escrever uma funcionalidade" e deixá-lo iterar cegamente em arquivos de código (Vibe Coding) gera o **Flow-Debt** (Dívida de Fluxo) e degradações arquiteturais silenciosas.

Todo o desenvolvimento **deve** obedecer ao pipeline de **Spec-Driven Development (SDD)**. O Agente está expressamente proibido de gerar código-fonte sem antes:

1. Validar a intenção contra o `manifesto_soda.md`.
2. Escrever e aprovar um artefato estático e imutável de planejamento (`proposal.md`, `design.md`, `tasks.md`).
3. Somente após a validação humana ou de um _Red Teamer Agent_ sobre as especificações, o agente recebe a permissão para iniciar a escrita do código em Rust ou React.

## 2. O PROTOCOLO BMAD E SHADOW WORKSPACES

A base principal do código (`main` branch) é sagrada. O SODA opera sob o **First Draft Protocol**, no qual a Inteligência Artificial nunca possui permissão direta de commit no ambiente de produção. O fluxo operacional exige o cumprimento do protocolo **BMAD (Branch, Mutate, Approve, Diff)**:

- **B - Branch (Shadow Workspace):** O Agente inicia a tarefa criando uma ramificação isolada e invisível ao ambiente principal. É proibido testar ou compilar hipóteses diretamente na raiz do usuário.
- **M - Mutate:** O código é escrito, refatorado e submetido à prova irrefutável do _Borrow Checker_ do compilador Rust (`cargo check` / `cargo test`).
- **A - Approve (Human-In-The-Loop Dinâmico):** Nenhum _Merge_ é feito cegamente. O Agente calcula o "Raio de Explosão" (Blast Radius) das suas alterações e invoca um _breakpoint_ assíncrono. O usuário 2e/TDAH revisa apenas o impacto visualizado, poupando fadiga cognitiva.
- **D - Diff:** A inserção das modificações na `main` branch através de um rebase semântico limpo, operado matematicamente sem gerar _Merge Commits_ poluídos.

## 3. ANTI-CONSENSO E "CONSENSUS-FREE MAD"

Quando múltiplos agentes (ex: um Agente de UI e um Agente de Código Rust) discordam sobre uma abordagem, ou quando os testes locais falham repetidamente (Ralph Loop), os Agentes possuem uma tendência perigosa de forjar um "falso consenso" para encerrar a tarefa, gerando código defeituoso.

O SODA implementa o **Debate Multi-Agente Anti-Consenso (Consensus-Free MAD)**:

- A instrução do agente incorpora um viés de _Pessimismo da Razão_.
- Se houver falha de validação ou violação das regras de 6GB de VRAM, os agentes **estão proibidos** de "chegar a um acordo médio". Eles devem compilar as visões divergentes, expor a falha arquitetural (Falsification Testing) e submeter o impasse à decisão explícita do Arquiteto Humano ou do Orquestrador Mestre.

## 4. A DOUTRINA DA CANIBALIZAÇÃO E O GIT SUBREPO

Sistemas baseados em nuvem resolvem dependências baixando ecossistemas inteiros via `npm install` ou `pip`. Essa prática introduz _Dependency Bloat_, lixo tóxico computacional e daemons em background, o que é letal para as restrições da RTX 2060m e do i9.

A assimilação de inteligência Open-Source no SODA obedece à tática da **Canibalização Cirúrgica**:

1. **O Fim do Git Submodule:** É proibido o uso de `git submodule` ou `git subtree`, pois fracionam o histórico ou corrompem o versionamento em modo WAL.
2. **Uso Exclusivo do Git Subrepo:** A incorporação de dependências externas deve ser feita via ferramenta `git subrepo`. Ela permite copiar o artefato para a árvore local de forma plana, mantendo os metadados de sincronização ocultos (`.gitrepo`).
3. **Extração de Ouro, Descartes de Lixo:** Ao incorporar um projeto via _Subrepo_, o Agente deve instanciar ferramentas AST (como _Tree-Sitter_ via _JCodeMunch MCP_) para dissecar o repositório, reescrever a "alma matemática" nativamente em Rust e **apagar impiedosamente** os servidores Express, Node.js, interfaces Chromium ou painéis FastAPI que o acompanham. O SODA consome apenas a lógica e descarta o bloatware.

## 5. QUALITY GATES E INTEGRIDADE DE ENCERRAMENTO

Uma tarefa só se considera finalizada quando os _Quality Gates_ automatizados em Rust passarem (Exit Code `0`). A IA não possui a prerrogativa de declarar sucesso baseado na plausibilidade textual de sua resposta. Se o `cargo build` rejeitar o código, a máquina de estados deve ser reiniciada no _Shadow Workspace_ sem poluir a visão do usuário, até que a matemática da compilação prove a correção da arquitetura.

_Fim do Documento de Governança. Os agentes agora estão confinados pelas leis metodológicas da engenharia de precisão._