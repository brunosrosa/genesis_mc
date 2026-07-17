---
id: "ADR-031"
title: "ADR-031-Harvester-Anatomia-11-Blobs-e-Leis-Inegociaveis"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Formaliza o Harvester da Fase 0 como Trator Mecânico Zero-AI de Fotografia Completa: define as 4 Leis Inegociáveis da Execução e a anatomia tática dos 11 artefatos extraídos, garantindo que a IA das Fases 2-3 reconstrua o software mentalmente sem alucinações."
---

# ADR-031: Harvester — Anatomia dos 11 Blobs e as 4 Leis Inegociáveis da Execução

## Status
Aceito (Ativo, Inegociável e Fundacional para a Fábrica SODA V6)

## Contexto
O Harvester da Fase 0 (V6) é o primeiro componente puramente nativo em Rust a tocar o repositório alvo após o clone. Sua meta é **Fotografia Completa (Zero Truncamento)**: a união dos 11 artefatos extraídos deve permitir que a IA das Fases 2 e 3 reconstrua o software original **mentalmente**, expurgando o lixo. Sem esse contrato, qualquer Lente Cognitiva opera sobre uma fotografia furada e alucina para preencher lacunas.

Até a V6, a anatomia dos artefatos existia apenas como cânone oral, espalhada por comentários de PRs, drafts do NotebookLM e mensagens de chat. Não havia um documento técnico que vinculasse a meta da Fotografia Completa aos limites mecânicos que a tornam possível. Este ADR congela essas decisões **antes** que a próxima geração de lâminas SAST seja escrita, garantindo que nenhum desvio reintroduza o *Fail-Soft mascarado* (erro de extração escondido por "0 bytes + log de warning") ou o *timeout cego* (180s fixos que trucam árvores úteis).

## Decisão Arquitetural

### 1. Identidade: O Trator Mecânico Zero-AI

O Harvester **NÃO** é uma IA que lê código. É um **trator mecânico** que arranca metadados do terreno com lâminas de parser nativas (Rust) e sidecars SAST isolados (Landlock/AppContainer). Cada lâmina tem uma missão tática isolada e **nunca vaza para o domínio da outra**. A IA só entra em ação nas Fases 2 e 3, consumindo os artefatos como SSOT.

**Implicação direta:** zero dependência de LLM, embedding remoto, ou "smart" heuristic durante a extração. Tudo é `cargo build`, regex, tree-sitter, ast-grep, opengrep, clippy, biome, oxc, govulncheck ou `reqwest` para GitHub API.

### 2. Meta Inegociável: Fotografia Completa (Zero Truncamento)

Cada blob deve carregar a **totalidade do DNA** do seu domínio, dentro dos limites de filtragem declarados. Truncar para "caber" em tokens é proibido. Se um blob estoura o limite físico de I/O, o Harvester **falha fechado** (ver Lei IV) — nunca entrega uma fotografia parcial.

### 3. As 4 Leis Inegociáveis da Execução

#### Lei I — Radar Global e Poda Universal
Antes de qualquer extração, o Harvester varre o repositório para mapear a **linguagem dominante** e extirpar preventivamente o lixo do diretório efêmero:
- `node_modules/`, `target/`, `dist/`, `build/`, `.venv/`, `vendor/`
- `tests/` cegos, `**/mocks/*`, `__snapshots__/`
- Arquivos minificados (< 7% de espaço em branco — ADR-024 §C)
- Binários, imagens, fontes, vídeos

A extração **só lê terreno limpo**. Esta lei torna o Lei III (consciência de monorepo) eficiente: sem lixo, o custo da varredura recursiva cai ordens de magnitude.

#### Lei II — Timeouts Elásticos
Fim do timeout fixo de 180s. O tempo de uma extração é **proporcional ao sucesso**: se um parser nativo está lendo uma árvore complexa e útil, o tempo se estende até o **sucesso absoluto** ou o limite físico de I/O. Sidecars SAST que suportam `--allow-rule-timeout-control` (OpenGrep) **devem** usá-lo (ADR-024 §A). O limite é o sucesso, não o relógio.

#### Lei III — Consciência de Monorepo (Cross-ref ADR-025)
Linters pesados (govulncheck, biome, opengrep) **não podem atirar às cegas na raiz**. Devem:
1. Realizar o **Pre-flight Check** que detecta onde estão os manifestos raiz (`go.mod`, `Cargo.toml`, `package.json`, `mix.exs`).
2. Executar a partir do `cwd` correto (subdiretório do manifesto).
3. Usar alvos recursivos explícitos (`./...` em Go, `**/*.go` em JS) em vez de varredura cega.

Esta lei é a contraparte positiva da ADR-025: a ADR-025 decretou a **proibição**, esta lei decreta a **obrigação** da topologia correta.

#### Lei IV — Lei do Zero-Byte Uniforme
Nenhuma ferramenta pode gravar `"Warning: Timeout"` ou `"Erro: 0 matches"` no payload salvo no SQLite. A escolha é binária:
- **Caso A (sucesso absoluto):** 100% da verdade foi extraída, DNA puro, payload completo gravado.
- **Caso B (qualquer falha):** o payload gravado é **0 bytes** (registro existe para trilha auditável, mas o conteúdo é vazio) e a flag `status_atualizacao` na tabela `repositorios` é marcada como `ERRO_F0` (cross-ref ADR-019).

**Não há meio-termo.** Não há "extraí 70% então gravei warning". A escolha reflete a Pessimismo da Razão: o silêncio absoluto (0 bytes) é honesto; um warning parcial é mentira.

### 4. A Anatomia Cirúrgica dos 11 Blobs

Cada blob é uma **lâmina tática isolada** com missão própria. O arsenal é puramente nativo em Rust (sem wrappers frágeis de APIs externas, exceto onde declarado).

| # | Blob | Apelido | Lâmina Tática | Missão | Artefato |
|---|---|---|---|---|---|
| 1 | `blob_01_promessa_readme` | O "Pitch" Original | Parser Markdown nativo | Visão do autor via `README.md`, expurgando links mortos e imagens, preservando texto e intenção intactos | Texto Markdown puro |
| 2 | `blob_02_dependency_manifest` | A Obesidade Sistêmica | Concatenador de TOML/JSON/YAML | DNA da cadeia de suprimentos: `Cargo.toml` + `package.json` + `pyproject.toml` + `go.mod` + `mix.exs` + locks correspondentes. Revela autossuficiência ou lixo tóxico embarcado (Node.js/Electron) | Texto estruturado |
| 3 | `blob_03_test_intent` | A Intenção Blindada | AST tree-sitter/ast-grep | Assinaturas das suítes de teste (`tests/`) — sem o miolo. Revela a ontologia das regras de negócio que o software **de fato** protege em produção | Outline O(1) |
| 4 | `blob_04_repo_outline` | A Alma Matemática O(1) | AST tree-sitter/ast-grep | **Apenas** assinaturas: funções, classes, traits, impls, enums. Implementação interna é **podada** cirurgicamente. Retorna o esqueleto puro para transmutação agnóstica de hardware | Outline O(1) |
| 5 | `blob_05_architecture_map` | A Topologia Espacial | Parser de imports em RAM | Grafo de dependências e pastas gerado **na RAM**, persistido como adjacência. Responde: "como os arquivos se importam e se conectam?" — distingue monólito acoplado de arquitetura modular | Grafo + lista |
| 6 | `blob_06_unsafe_hotspots` | A Cicatriz de Segurança | Bisturi SAST (opengrep, clippy, biome) | Foco implacável em dívida técnica **severa**: blocos `unsafe {}`, ponteiros crus, `eval()`, `exec()` arbitrário, hardcoded keys, gambiarras de segurança, injeção, XSS, SQLi. Filtra estética; mira perfuração | Texto-relatório tático |
| 7 | `blob_07_ops_blueprint` | A Fricção de DevOps | Capturador de CI/CD | `Dockerfile*`, `.github/workflows/*`, `Makefile`, `docker-compose*.yml`, `Jenkinsfile`, `.gitlab-ci.yml`. Captura **integral** (não resumida). Revela se o projeto exige infra pesada ou compila limpo | Texto |
| 8 | `blob_08_health_report` | A Podridão Estrutural | Mesmo motor SAST do Blob 06, com flag `--skip-formatter` | Ignora estética (espaços, indentação). Relata **pura complexidade ciclomática**, código morto, code smells, entropia em runtime. É o Blob 06 sem o filtro de beleza | Texto-relatório tático |
| 9 | `blob_09_community_meta` | A Vitalidade Real | `reqwest` para GitHub REST API | Lead time de fechamento de Issues, PRs ativos, estrelas, data do último commit, taxa de release. Diferencia projetos maduros de abandonware. **Único blob com dependência de rede autorizada** | JSON |
| 10 | `blob_10_soda_canon_context` | As Leis Duras | Leitura local air-gapped | Carrega o `soda_canon_grounding.md` (ou equivalente canon snapshot). É a **Constituição SODA** anexada a cada dossiê para ancorar a IA das Fases 2-3 e impedir alucinações de hardware | Texto |
| 11 | `blob_11_ux_contracts` | A Mecânica Visual | Oxc parser (Svelte/React/TS) | Extração cirúrgica de UI: extirpa **100% do HTML e CSS** (lixo estético) e devolve **estritamente a Mecânica de Estados** — Props (entradas) e Events/Dispatchers (saídas) que governam a previsibilidade do componente | Outline O(1) |

**Princípio de não-vazamento:** os 11 blobs são ortogonais. O Blob 04 (outline) **nunca** inclui lógica de teste (vai para Blob 03). O Blob 06 (security) **nunca** inclui complexidade (vai para Blob 08). O Blob 09 (community) **nunca** inclui código (vai para Blobs 04, 06, 08). Cada lâmina cumpre sua missão e **delega o resto ao vizinho correto**.

## Consequências

### Positivas
- **Fotografia Completa Garantida:** as 4 Leis Inegociáveis blindam o Harvester contra truncamento preguiçoso, Fail-Soft mascarado, e timeout cego. A IA das Fases 2-3 sempre recebe 100% do DNA ou 0 bytes honestos.
- **Agnosticismo de Hardware por Construção:** os Blobs 03, 04, 05, 11 (outline-only) são transmutáveis para qualquer stack (Rust, Go, Python, TS) sem lock-in de vendor — prepara a expansão para CubeCL/Burn.
- **Detecção Precoce de Lixo Tóxico:** o Blob 02 entrega a "obesidade sistêmica" do projeto. A Lente C consegue classificar imediatamente se um candidato traz Node.js/Electron embarcado.
- **Constituição Anexada:** o Blob 10 transforma o canon em constante do pipeline, garantindo que nenhuma Lente "esqueça" o agnosticismo de hardware ou a Pessimismo da Razão.

### Negativas
- **Custo de I/O Inicial:** a Lei I (Radar Global + Poda) adiciona 1 varredura completa do FS antes da extração. Em monorepos de 5GB+, isso pode levar 30-60s. Mitigação: cache por `commit_sha` no SQLite (Lei IV já exige 0 bytes em falha, o cache é trivial).
- **Complexidade de Orquestração:** 11 lâminas com 4 Leis + cross-refs com ADR-019, ADR-024, ADR-025 exigem um state machine explícito (já provido por ADR-019). Sem o ADR-019, este ADR seria ingovernável.
- **Pressão sobre o time:** nenhuma nova lâmina SAST pode ser adicionada sem passar pelo filtro das 4 Leis. Lentidão desejada — é o preço do "fail-closed honest".

## Cross-References
- **ADR-019** (Máquina de Estados ETL): provê os enums físicos (`F0_OK`, `DEGRADADO_F0`, `ERRO_F0`) que esta ADR referencia na Lei IV.
- **ADR-024** (Engenharia Performance SAST): provê a base da Lei II (Timeouts Elásticos) e Lei I (Poda Universal — 7% whitespace).
- **ADR-025** (Consciência de Monorepos): provê a base da Lei III (Pre-flight Check + alvos recursivos).

## Restrições Bare-Metal
- **Zero LLM no caminho de extração:** nenhuma chamada a OpenAI, Gemini, Claude, ou modelo local durante a Fase 0. A IA só lê os artefatos nas Fases 2-3.
- **Zero Mentira no SQLite:** ou 100% verdade, ou 0 bytes. Nunca warning truncado.
- **Zero Timeout Cego:** timeouts são sempre adaptativos ou eliminados. O relógio não governa a extração; o sucesso governa.
- **Zero Lockfile Ignorado:** o Blob 02 inclui **sempre** os lockfiles (cross-ref ADR-024 §B). Filtros podem amputar `tests/`, mas `Cargo.lock` e `package-lock.json` são sagrados.
