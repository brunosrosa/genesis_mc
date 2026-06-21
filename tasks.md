# Tasks Monorepo SAST

- [ ] Adicionar descoberta rapida de manifestos com exclusao de `.git`, `node_modules`, `target`, `venv` e `dist`.
- [ ] Estender o executor do sandbox para aceitar `cwd` por subprojeto sem perder os timeouts adaptativos.
- [ ] Mapear cada lamina para manifestos compativeis e consolidar resultados por repositorio.
- [ ] Governar subexecucoes com `tokio::sync::Semaphore::new(3)`.
- [ ] Cobrir com testes focados de descoberta, `cwd`, agregacao e limite de concorrencia.
- [ ] Validar com `cargo check` e `cargo run --bin f0_harvester_cli -- --repo mendableai/firecrawl --direct`.
