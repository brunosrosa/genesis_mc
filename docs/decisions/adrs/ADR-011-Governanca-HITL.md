---
id: "ADR-011"
title: "ADR-011-Governanca-HITL"
version: 2.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Requer a aprovação humana (Human-In-The-Loop) interativa em sessão ativa via chat/CLI, aplicando a Regra de Pragmatismo de Interface para contornar UIs gráficas enquanto a Milestone 4 estiver inativa."
---

# ADR-011: Governança Human-In-The-Loop (HITL) Interativa e Pragmatismo de Interface

## Status
Aceito (Ativo, Inegociável e Fundacional para Arquitetura SOULS V6)

## Contexto Técnico e a Necessidade de Controle Soberano
Permitir que agentes autônomos de IA alterem arquivos diretamente no disco de desenvolvimento ou realizem commits sem supervisão na branch principal gera corrupção silenciosa de dados (SDC) e perda de alinhamento de intenção. 
Contudo, depender de interfaces gráficas complexas (Agent Inbox e Blast Radius Canvas) antes do amadurecimento do frontend (Milestone 4) paralisa o fluxo de entrega e impõe fricção inútil. É necessário garantir o controle humano absoluto sem criar bloqueios de infraestrutura visual inexistente.

## Decisão Arquitetural (O Protocolo BMAD e o Pragmatismo HITL via Chat/CLI)
Fica estabelecido o protocolo **BMAD (Branch, Mutate, Approve, Diff)** com a **Regra de Pragmatismo de Interface**:

### 1. Regra de Pragmatismo de Interface (Interrupção via Chat / Sem UI Gráfica)
*   Enquanto a Milestone 4 (Frontend Canvas) não estiver ativa em produção, o SOULS contorna compulsoriamente a dependência de elementos visuais gráficos como "Agent Inbox" e "Blast Radius Canvas".
*   Toda e qualquer aprovação HITL ou interrogação epistêmica de ambiguidade DEVE ocorrer **INTERATIVAMENTE EM SESSÃO ATIVA**.
*   O daemon Rust e as garras MCP devem pausar assincronamente a execução do Tokio, imprimir o diff do código no chat/stdout e interrogar o usuário em modo CLI de perguntas poderosas (Rapport Socrático sem "Por que").
*   O agente aguardará passivamente o input textual do operador humano na própria thread de chat ativo antes de aplicar alterações ou rebase no disco físico.

### 2. Estágios do Protocolo BMAD
1.  **B - Branch (Isolamento Físico):** Qualquer mutação é iniciada isolando a tarefa em um **Shadow Workspace** atômico criado em tempo constante $\mathcal{O}(1)$ utilizando links físicos rígidos (*snapsafe*), consumindo zero bytes adicionais do disco hospedeiro.
2.  **M - Mutate (Escrita Atômica Protegida):** O código é alterado por meio de escritas atômicas baseadas em Mutex concorrentes em Rust (`file_locker.rs`). As alterações passam pela alfândega de compilação obtendo obrigatoriamente Exit Code 0.
3.  **A - Approve (Interrupção Socrática no Chat):** O agente consolida o Blast Radius (arquivos tocados e diff formatado), imprime o payload na janela de chat ativa e realiza a pausa síncrona/assíncrona da runtime Tokio, aguardando a resposta afirmativa do operador na CLI.
4.  **D - Diff (Rebase Semântico):** Uma vez aprovado pelo operador no chat, o core em Rust realiza um **Rebase Semântico** direto e atômico em direção à branch de produção via `gitoxide`, eliminando a ramificação temporária. Fica expressamente proibido o uso de *Merge Commits* poluentes.

## Prevenção de Colisões Concorrentes de Escrita
- **Gerenciador de Travas `file_locker.rs`:** Torna-se obrigatório o uso do gerenciador centralizado de travas `file_locker.rs` para toda operação de Mutate.
- **Sequenciamento Concorrente:** Toda escrita concorrente iniciada por subagentes no background deve ser estritamente sequenciada por um `Mutex` assíncrono do Tokio indexado ao caminho físico do arquivo e gerenciado estaticamente em um `OnceLock` contendo um `DashMap`.
- **Higiene de RAM Host e Limpeza de Chaves Órfãs:** Para evitar vazamentos de memória na RAM host, o `file_locker.rs` deve varrer o `DashMap` e limpar chaves órfãs cuja contagem de referências fortes do `Arc` seja igual a 1 (`Arc::strong_count(&lock) == 1`) antes de liberar o escopo de execução.
- **Mitigação de Silent Data Corruption (SDC):** Toda modificação de arquivo deve obrigatoriamente passar por swap temporário e escrita atômica via `atomic-write-file` para erradicar corrupção silenciosa de dados durante picos de I/O ou desligamentos repentinos.

## Consequências Operacionais
- **Soberania e Agilidade:** O humano retém o controle absoluto sobre as mutações em disco através do chat/CLI ativo, com zero dependência de componentes visuais gráficos pendentes de implementação.
- **Erradicação do SDC:** Mutações indesejadas são barradas antes de tocar nos caminhos cruciais de produção.

## Restrições Bare-Metal
- **Blast Radius Trigger:** Modificações de middlewares de segurança ou banco de dados principal exigem aprovação HITL explícita no chat.
- **Isolamento de Shadow Workspace:** O tempo constante de instanciação do snapsafe via links físicos no host deve ser de no máximo **50ms**.
- **I/O fora do Event Loop:** Operações do **gitoxide** (Rebase Semântico, snapshots, verificação de drift) são proibidas na thread principal do Tokio; devem ser descarregadas para `tokio::task::spawn_blocking` ou *background workers* comunicando-se via **MPSC**.
