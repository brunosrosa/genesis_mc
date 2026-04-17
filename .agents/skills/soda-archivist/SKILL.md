---
name: soda-archivist
description: O Faxineiro Semântico e Guardião de Estado. Gerencia o Frontmatter State Tracking para liberar VRAM e aplica a Poda Sináptica (Esquecimento Geométrico) para mover lógicas mortas ao armazenamento frio, erradicando o Context Rot.
triggers: ["soda-archivist", "limpar rascunhos", "atualizar estado", "frontmatter", "arquivar tarefa", "esquecimento geométrico", "/archive", "faxina semântica"]
---

# Skill: SODA Archivist v2.0 (O Guardião de Estado e Faxineiro Semântico)

## Goal
Erradicar a "Dívida de Fluxo" (*Flow-Debt*) e a amnésia sistêmica (*Context Rot*) durante o ciclo de vida do desenvolvimento. O SODA Archivist atua em duas frentes fundamentais: **Gestão de Estado Sem Servidor** e **Garbage Collection Cognitivo**. A IA está terminantemente proibida de usar sua restrita janela de contexto (VRAM) para "lembrar" em qual etapa do planejamento o projeto está [1]. O objetivo desta skill é gravar o estado iterativo diretamente no disco (Frontmatter) e aplicar o "Esquecimento Geométrico" (Semantic Decay) em rascunhos, protegendo a sobrecarga mental do usuário (2e/TDAH) e o hardware da máquina [1, 2].

## Instructions
Sempre que uma iteração for concluída, uma hipótese for invalidada ou o comando explícito `/archive` for invocado pelo usuário, você DEVE executar as seguintes fases em ordem estrita [2, 3]:

1. **Frontmatter State Tracking (Gravação de Estado em Disco):** 
   - Abra o arquivo de planejamento ativo (ex: `Project Brief` ou `tasks.md`).
   - Atualize OBRIGATORIAMENTE os metadados de execução no cabeçalho YAML (`Frontmatter`), ajustando chaves como `currentPhase`, `stepsCompleted` e `active_focus` [1]. 
   - Após a gravação, você está autorizado a "zerar" mentalmente o seu contexto sobre os passos anteriores.
2. **Sincronização de Deltas (SSOT):** 
   - Transmute as conclusões técnicas da tarefa recém-finalizada e faça o *merge* de forma enxuta nos documentos canônicos da arquitetura do projeto (ex: `ARCHITECTURE.md` ou o manifesto global), mantendo a Única Fonte da Verdade (SSOT) impecável [3].
3. **Poda Sináptica e Esquecimento Geométrico (Semantic Decay):** 
   - Identifique lógicas mortas, "hipóteses falhas", *scratchpads* (blocos de rascunho) ou ideias rotuladas como "talvez no futuro" [2, 3]. 
   - NÃO as deixe na árvore principal poluindo a visão. Mova esses artefatos para o diretório de armazenamento frio (`.archive/`) [2].
4. **O Expurgo Implacável:** 
   - Arquivos temporários e logs inúteis que não possuem valor histórico devem ser sumariamente deletados [3].
5. **Relatório Silencioso:** 
   - Conclua a operação de forma espartana. Não exiba na interface o conteúdo dos textos que você apagou ou moveu, gerando apenas uma confirmação atômica de que o estado foi atualizado e a árvore higienizada [3].

## Constraints
* **PROIBIÇÃO DE ALUCINAÇÃO DE ESTADO:** Nunca tente adivinhar a próxima etapa de um projeto baseando-se no histórico da conversa do chat. Leia sempre o cabeçalho YAML (Frontmatter) do projeto [1].
* **PRESERVAÇÃO PASSIVA:** O esquecimento geométrico garante que as ideias antigas decaiam para o "armazenamento frio" (pasta `.archive/`), protegendo o usuário de TDAH do medo de perder "ideias geniais antigas", mas removendo-as da linha de visão imediata [2].
* **ZERO MICRO-GERENCIAMENTO:** Não peça permissão ao usuário para deletar arquivos temporários gerados pela própria IA durante testes de compilação [3].

## Examples

**Entrada do Usuário:** 
"A refatoração do IPC no Tauri foi concluída com sucesso e aprovada pelo compilador. Roda o archivist pra gente ir pro próximo passo."

**Ação do Agente:**
1. O agente atualiza o Frontmatter do arquivo `design.md`: muda `stepsCompleted: 3` para `4` e `currentPhase` para `"Integração Wasmtime"` [1].
2. Adiciona o novo fluxo IPC ao `docs/ARCHITECTURE.md` [3].
3. Move o arquivo de tentativa frustrada `rascunho_ipc_v1.rs` para `.archive/rascunho_ipc_v1.rs` (Semantic Decay) [2].
4. Deleta o `test_log_tmp.txt` [3].
5. Responde: *"Estado atualizado no Frontmatter (Passo 4 iniciado). Deltas consolidados e lógicas falhas arquivadas. Workspace limpo."*