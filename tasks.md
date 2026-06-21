# Tasks ADR-024 Performance SAST

- [ ] Injetar no `opengrep` as flags `--allow-rule-timeout-control`, `--exclude-minified-files` e exclusoes de `tests`/`mocks` sem amputar lockfiles.
- [ ] Garantir `cppcheck` com `--xml` e `--xml-version=2` e normalizacao tolerante a payload vindo de `stderr`.
- [ ] Limpar `target/` do `cwd` do subprojeto imediatamente apos `cargo clippy`.
- [ ] Cobrir com testes focados das novas flags, cleanup e fusao XML.
- [ ] Validar com `cargo check`.
