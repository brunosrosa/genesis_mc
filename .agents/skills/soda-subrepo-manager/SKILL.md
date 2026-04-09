---
name: soda-subrepo-manager
description: Gerencia a integração, atualização e validação autônoma de código de terceiros e repositórios externos no Genesis MC utilizando estritamente a ferramenta Git Subrepo, com foco em DevSecOps e rollbacks instantâneos.
triggers: ["puxar dependência externa", "atualizar subrepo", "integrar código de terceiros", "clonar subrepositório"]
---

# Diretrizes da Habilidade: Gestor de Integração Git Subrepo (soda-subrepo-manager)

Você assume o papel de um Arquiteto DevSecOps no ecossistema **Genesis MC (SODA)**. É terminantemente proibido utilizar `git clone` direto, `git submodule` ou `git subtree` para integrar código de terceiros. Você deve utilizar **exclusivamente** o `git subrepo`.

Sempre que invocado, você DEVE seguir obrigatoriamente as 4 Fases de Execução abaixo:

### Fase 1: Preservação de Estado via Snapshot Local
Antes de qualquer mutação, valide a integridade do branch atual com `git status`. Em seguida, crie um branch de backup órfão usando o timestamp UNIX para garantir um ponto de restauração:
`BACKUP_BRANCH="snapshot-$(date +%s)"`
`git branch $BACKUP_BRANCH`

### Fase 2: Instanciação Operacional da Atualização
Realize a assimilação determinística do código utilizando o Git Subrepo:
`git subrepo pull <caminho_do_subdiretorio>`
Vigie incondicionalmente os *exit codes*. Se ocorrerem conflitos severos de mesclagem que exijam intervenção manual complexa, aborte a operação e salte IMEDIATAMENTE para a Fase 4 (Rollback).

### Fase 3: Bateria de Testes de Quebra (Integração Shift-Left)
Identifique e invoque as suítes de testes fundamentais pertinentes à camada modificada (ex: `cargo check`, `cargo test`, `npm run test` ou linters de segurança).
* **Caminho Feliz:** Se os testes passarem (exit code 0), exclua a branch de snapshot (`git branch -D $BACKUP_BRANCH`) e declare o sucesso da integração.
* **Caminho Fatal:** Qualquer falha, *warning* crítico de linter ou colapso estrutural significa que o código externo é tóxico. Dirija-se AGRESSIVAMENTE para a Fase 4.

### Fase 4: Reconciliação Punitiva (Rollback Instantâneo)
Diante de qualquer falha na Fase 2 ou Fase 3, reverta o repositório para o estado imaculado:
1. `git reset --hard $BACKUP_BRANCH`
2. `git clean -fd`
3. `git branch -D $BACKUP_BRANCH`
Após a limpeza destrutiva, informe ao usuário de forma clara que a integração falhou nos portões de qualidade e que o sistema foi totalmente revertido para a integridade original sem deixar resíduos.
