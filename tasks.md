# Tasks Timeout Opengrep

- [ ] Autopsiar `opengrep` no `svelte` em shadow workspace local e registrar o padrao estrutural toxico real.
- [ ] Implementar a Camada A no `sidecar.rs` com `--exclude` organicos para lixo recorrente de monorepos JS.
- [ ] Implementar a Camada B com scoping dinamico nativo a partir das raizes uteis derivadas do AST/caminhos-fonte.
- [ ] Impedir que o algoritmo reinsira scopes pais toxicos quando houver filhos grandes e arquivos diretos no mesmo nivel.
- [ ] Cobrir arquivos diretos da ancora via alvos explicitos do `opengrep`, sem reabrir a subarvore inteira.
- [ ] Tirar `cargo clippy` do `target/` dentro do repo efemero e mandar cache/build para `.soda_sandbox`.
- [ ] Portar o planner AST do `opengrep` para `biome` e `oxlint`, respeitando fronteiras de subpackages `package.json`.
- [ ] Garantir que roots de monorepo com subpackages nao reabram `.` nem dupliquem cobertura de filhos.
- [ ] Cobrir a logica nova com testes focados do builder/selecionador de raizes.
- [ ] Rodar Ralph Loop: `cargo check` e teste seco contra `sveltejs/svelte` ate `exit code 0`.
- [ ] Promover `cargo clippy` para perfil de timeout ocioso profundo sem relaxar o timeout absoluto global.
- [ ] Corrigir o fetch de `blob_09_community_meta` para usar o `full_name` canonico retornado pelo GitHub antes das buscas de PR.
- [ ] Criar refresh cirurgico de `blob_09_community_meta` no SQLite para um `repo_id` isolado.
- [ ] Validar com teste focado do `github_tracker`, `cargo check` dos bins tocados e leitura do blob no DB.
- [ ] Provar por leitura de codigo que `validate_execution_root()` ja aceita subpastas fatiadas do repo e que `--config` do OpenGrep ja entra absoluto.
- [ ] Blindar `ensure_semgrep_rule_bundle()` contra corrida entre workers concorrentes do Tokio por repositorio/ruleset.
- [ ] Cobrir a materializacao concorrente/idempotente do bundle YAML com teste focado.
- [ ] Rodar `cargo clippy -- -D warnings` e `cargo test` com foco no harvester ate Exit Code 0.
