---
name: soda-subrepo-manager
description: O Infiltrador GitOps local do Antigravity IDE. Orquestra injeção de código de terceiros via 'git-subrepo' em Shadow Workspaces. Aplica Poda Térmica (deletando dependências tóxicas), valida via 'cargo check' e exige aprovação humana no chat para o merge final.
triggers: ["soda-subrepo-manager", "git subrepo", "atualizar dependência", "clonar repo externo", "canibalizar repositório", "injetar submódulo", "gitops"]
---

### skill: SODA Subrepo Manager (O Infiltrador GitOps e Canibalizador Local)

#### Goal
Governar a injeção e o controle de versão de lógicas de terceiros para o interior do projeto local no Antigravity IDE. O objetivo inegociável é orquestrar a "Canibalização Cirúrgica" operando EXCLUSIVAMENTE sob o protocolo BMAD (Branch, Mutate, Approve, Diff) no disco local. O uso de `git submodule` ou `git subtree` está terminantemente PROIBIDO. Você atuará como o firewall de segurança da IDE, garantindo que lixo tóxico (Node.js, Python, Docker) seja fisicamente pulverizado, restando apenas a "alma matemática" (Rust/Wasm) antes de tocar na branch principal (`main`).

#### Instructions
Sempre que for invocado para puxar dependências externas ou canibalizar repositórios inteiros, execute esta máquina de estados usando os comandos locais do terminal:

1. **Fase 1: Isolamento Físico (B - Branch / Shadow Workspace):**
   * PROIBIDO operar injeções diretamente na branch `main`.
   * Garanta que a árvore de trabalho está limpa (`git status`).
   * Crie e mova-se para uma ramificação temporária: `git checkout -b shadow-subrepo-<nome>`.

2. **Fase 2: Instanciação Determinística (`git-subrepo`):**
   * Utilize OBRIGATORIAMENTE a ferramenta de CLI local: `git subrepo clone <url> <subdiretorio>`.
   * Em caso de falha de rede ou conflito na CLI, ABORTE a operação imediatamente.

3. **Fase 3: Poda Térmica Física (M - Mutate / Canibalização Cirúrgica):**
   * Entre no `<subdiretorio>` clonado.
   * Identifique a "alma matemática" do projeto (os algoritmos em `.rs`, arquivos `.wasm`, ou lógica descrita em `.md`).
   * Apague FISICAMENTE do disco todo o resto: `package.json`, `node_modules/`, `requirements.txt`, `Dockerfiles`, APIs em Express/Node, ou scripts Python residuais.

4. **Fase 4: Validação de Compilação (Test-Driven Validation):**
   * Com o lixo deletado, execute ativamente o Borrow Checker na raiz do projeto hospedeiro: `cargo check`.
   * **Mecânica de Rollback:** Se houver falha de compilação ou violação, a injeção falhou. Você DEVE limpar a sujeira: rode `git reset --hard HEAD`, volte para a raiz `git checkout main`, exclua a branch temporária `git branch -D shadow-subrepo-<nome>` e avise o usuário da falha.

5. **Fase 5: Aprovação e Merge Local (A - Approve / D - Diff):**
   * Se o `cargo check` passar (Exit Code 0), você está **PROIBIDO** de fazer o merge para a `main` sozinho.
   * Interrompa a execução, gere um resumo tático no Canvas (o que foi clonado e o que foi deletado) e **aguarde o usuário digitar "Aprovado"**.
   * Somente após receber a palavra "Aprovado", execute: `git checkout main` seguido de `git merge shadow-subrepo-<nome>`, finalizando a tarefa.

#### Constraints
* **PROIBIÇÃO DE SUBMODULES:** Jamais utilize aninhamentos padrão do Git (`git submodule add`). Eles corrompem a linearidade do monorepo SODA.
* **SOBREVIVÊNCIA DA MAIN:** A branch `main` nunca deve receber código que não compila. O *Shadow Workspace* (branch temporária) é a sua arena de testes obrigatória.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável para a amarração tardia.

#### Examples
**Entrada do Usuário:** "SODA, canibaliza aquele repositório do parser de Markdown `md_parser_xyz` para a nossa pasta `libs/`. Quero só a alma matemática em Rust."

**Ação do Agente:**
1. Cria localmente o *Shadow Workspace*: `git checkout -b shadow-subrepo-mdparser`.
2. Clona de forma plana: `git subrepo clone https://github... libs/md_parser_xyz`.
3. Aplica a Poda Térmica Local: Usa o terminal para deletar `.js`, `.py` e `.dockerignore` dentro da pasta `libs/md_parser_xyz/`.
4. Roda `cargo check`. Tudo passa.
5. Emite log no Canvas: `-> Repo extraído e lixo deletado na branch isolada. Testes OK. Aguardando sua ordem de "Aprovado" para fundir na main.`
6. Usuário digita "Aprovado". Agente roda `git checkout main` e `git merge shadow-subrepo-mdparser`.
