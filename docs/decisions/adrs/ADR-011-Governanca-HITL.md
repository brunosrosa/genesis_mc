---
id: "ADR-011"
title: "ADR-011-Governanca-HITL"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Requer a aprovação humana (Human-In-The-Loop) para operações destrutivas ou com alto Blast Radius."
---

# ADR-011-Governanca-HITL

## Status
Aceito (Ativo e Inegociável)

## Contexto
Permitir que agentes autônomos de IA mutem arquivos diretamente no disco principal de desenvolvimento ou realizem commits automáticos sem supervisão na branch principal (`main`) gera corrupção silenciosa de dados (SDC) e desorientação espacial do usuário. Além disso, mutações agressivas e inesperadas no código forçam reflows térmicos e confusão cognitiva imediata no usuário final, que perde o senso de controle de sua própria máquina.

## Decisão
Impor o protocolo **BMAD (Branch, Mutate, Approve, Diff)** para toda e qualquer alteração de código ou de dados na base principal do SOULS:
1. **B - Branch (Isolamento Físico):** Qualquer mutação é iniciada isolando a tarefa em um **Shadow Workspace** atômico criado em tempo constante $\mathcal{O}(1)$ utilizando links físicos rígidos (*snapsafe*), consumindo zero bytes adicionais do disco hospedeiro.
2. **M - Mutate (Escrita Atômica Protegida):** O código é alterado por meio de escritas atômicas baseadas em Mutex concorrentes em Rust. As alterações passam pela alfândega de compilação obtendo obrigatoriamente Exit Code 0.
3. **A - Approve (A Agent Inbox):** O agente consolida o **Blast Radius** (arquivos tocados e impactos previsíveis) e o envia como sugestão passiva para a **Agent Inbox** na interface Svelte 5. O sistema aguarda a aprovação explícita do usuário em modo **Human-In-The-Loop (HITL)**. A aprovação dispara a transição estética **Glow Revelation Transition** (brilho térmico suave nas bordas da janela sem reflow).
4. **D - Diff (Rebase Semântico):** Uma vez aprovado pelo humano, o core em Rust realiza um **Rebase Semântico** direto e atômico em direção à branch de produção, eliminando a ramificação temporária. Fica expressamente proibido o uso de commits de mesclagem convencionais (*Merge Commits*) que poluem a árvore histórica do projeto.

## Prevenção de Colisões Concorrentes de Escrita

- **Gerenciador de Travas `file_locker.rs`:** Torna-se obrigatório o uso do gerenciador centralizado de travas `file_locker.rs` para toda operação de Mutate.
- **Sequenciamento de Escrita Concorrente:** Toda escrita concorrente iniciada por múltiplos subagentes agindo no background do Souls MC deve ser estritamente sequenciada por um `Mutex` assíncrono do Tokio indexado ao caminho físico do arquivo e gerenciado estaticamente em um `OnceLock` contendo um `DashMap`.
- **Higiene de RAM Host e Limpeza de Chaves Órfãs:** Para evitar vazamentos de memória monotônicos na RAM do host devido ao acúmulo de chaves de arquivos analisados ao longo de execuções contínuas, o `file_locker.rs` deve varrer o `DashMap` e limpar chaves órfãs cuja contagem de referências fortes do `Arc` seja igual a 1 (`Arc::strong_count(&lock) == 1`) antes de liberar o escopo de execução.
- **Mitigação de Silent Data Corruption (SDC):** Toda modificação de arquivo deve obrigatoriamente passar por swap temporário e escrita atômica via `atomic-write-file` para erradicar corrupção silenciosa de dados durante picos de I/O ou desligamentos repentinos.

## Consequências
- **Soberania do Usuário:** O humano retém o controle absoluto sobre o silício e o repositório físico da aplicação.
- **Erradicação do SDC:** Mutações indesejadas de IA são sumariamente barradas antes de tocar nos caminhos cruciais de produção.
- **Transparência de Impacto:** O Blast Radius fornece uma visualização imediata do alcance das modificações propostas no sistema, mitigando a fadiga de auditoria.

## Restrições Bare-Metal
- **Blast Radius Trigger:** Modificações de middlewares de segurança ou banco de dados principal exigem HITL obrigatório incondicional.
- **Glow Revelation Transition Performance:** A animação de aprovação na UI Svelte 5 deve rodar com taxa estável de **60 FPS** atrelada a transformações por GPU (iGPU), com duração máxima de **1500ms**.
- **Isolamento de Shadow Workspace:** O tempo constante de instanciação do snapsafe via links físicos no host deve ser de no máximo **50ms**.
- **I/O e Criptografia fora do Event Loop:** Operações do **gitoxide** (Rebase Semântico, snapshots, verificação de drift) e cálculos criptográficos pesados (**SHA-256**) são proibidos na thread principal do Tokio; devem ser descarregados para `tokio::task::spawn_blocking` ou *background workers* dedicados comunicando-se via **MPSC**, prevenindo starvation do Event Loop.
