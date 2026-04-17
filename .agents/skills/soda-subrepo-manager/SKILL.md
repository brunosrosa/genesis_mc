---
name: soda-subrepo-manager
description: O Infiltrador e Gestor de GitOps do SODA. Orquestra a injeção de código de terceiros usando estritamente 'git subrepo'. Aplica Snapshots de segurança, Testes de Quebra e Rollback autônomo (Shift-Left DevSecOps).
triggers: ["soda-subrepo-manager", "git subrepo", "atualizar dependência", "clonar repo externo", "canibalizar repositório", "injetar submódulo", "gitops"]
---

# Skill: SODA Subrepo Manager (O Infiltrador GitOps)

## Goal
Governar as permutas e o controle de versão bidirecional de repositórios não inatos aos domínios do projeto base do SODA. O objetivo é orquestrar a injeção de dependências sem poluir o histórico principal de commits e sem utilizar arquiteturas frágeis de ponteiros. A adoção de ferramentas obsoletas ou perigosas como `git submodule` ou `git subtree` está terminantemente PROIBIDA. Toda operação deve utilizar a ferramenta `git-subrepo`.

## Instructions
Sempre que for invocado para conduzir modificações, puxar dependências externas ou canibalizar repositórios para o interior do projeto local, você DEVE honrar a seguinte máquina de estados em 4 fases estritas:

1. **Fase 1: Ponto de Restauração (Snapshot Local):**
   - Antes de iniciar a operação, registre o estado atual e intacto do repositório host. 
   - Execute o armazenamento da HEAD (`git rev-parse HEAD`) em uma variável efêmera local.

2. **Fase 2: Instanciação Operacional via Git Subrepo:**
   - Execute a assimilação determinística usando OBRIGATORIAMENTE o comando `git subrepo clone <url> <subdiretorio>` ou `git subrepo pull <subdiretorio>`.
   - Se ocorrerem conflitos severos de mescla que exijam interações manuais exaustivas, ABORTE. Transborde o controle analítico imediatamente para a Fase 4 (Rollback).

3. **Fase 3: Bateria de Testes de Quebra (Shift-Left DevSecOps):**
   - Uma vez que o código externo achatou as estruturas correntes, o projeto está em latência não validada.
   - Execute ativamente os testes da cadeia sintática do ecossistema hospedeiro (ex: `cargo check`, testes unitários, linters de segurança).
   - Se houver falha, erro no STDERR ou quebra da compilação: a injeção é julgada tóxica. Avance para a Fase 4. Se passar limpo, finalize com sucesso.

4. **Fase 4: Restauração Destrutiva (Rollback Rigoroso):**
   - Se qualquer passo da Fase 2 ou Fase 3 falhar, recue o sistema para a integridade exata capturada na Fase 1.
   - Execute uma limpeza estática baseada em destituições forçadas: `git reset --hard <HASH_FASE_1>` e `git clean -fd`.
   - Explicite a ocorrência do sinistro e da regressão de forma clara ao usuário.

## Constraints
* **PROIBIÇÃO DE SUBMODULES/SUBTREES:** Jamais utilize ferramentas do Git padrão para aninhamento que quebrem a continuidade linear de histórico do SODA.
* **SEM COMPLACÊNCIA DE ERRO:** Em hipótese alguma ignore os *exit codes* da Fase 3. Um código externo que não compila localmente é um código bloqueado.

## Examples
**Entrada do Usuário:** 
"SODA, precisamos puxar a última atualização daquele repositório de parser que colocamos na pasta `lib/parser_externo/`."

**Ação do Agente:**
1. O agente executa `git rev-parse HEAD` e guarda o Hash de segurança.
2. O agente invoca `git subrepo pull lib/parser_externo/`.
3. O agente aciona a compilação de teste `cargo test -p parser_externo`.
4. (Cenário A) Passa: "Atualização injetada e validada. Histórico Git achatado."
   (Cenário B) Falha: O agente roda `git reset --hard` e responde: "A atualização corrompeu a compilação local. Operação revertida com sucesso."