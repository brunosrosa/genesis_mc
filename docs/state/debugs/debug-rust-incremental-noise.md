# Debug Session: rust-incremental-noise

Status: MITIGATED

Symptom:
- Durante `cargo test` e `cargo run` no Windows, aparecem warnings de diretório incremental com `os error 5` e houve pelo menos um `rustc` ICE ligado ao fluxo incremental.

Hypotheses:
1. O ruído vem de concorrência de builds/testes acessando o mesmo `target\debug\incremental`.
2. O sandbox/antivírus/AppContainer está segurando arquivos `.rmeta` durante a finalização da sessão incremental.
3. O problema é um bug do `rustc 1.94.1` no Windows com incremental + multi-crate + Tauri.
4. O ruído não afeta binários finais, mas degrada confiabilidade e pode causar falhas intermitentes de compilação futura.
5. O projeto está misturando comandos paralelos sobre o mesmo `target`, amplificando lock contention.

Plan:
1. Limpar a instrumentação da sessão anterior já confirmada.
2. Coletar evidência de ambiente e configuração de build incremental.
3. Reproduzir o ruído de forma controlada com e sem `CARGO_INCREMENTAL`.
4. Determinar se o impacto é apenas cosmético, intermitente ou bloqueante.

Evidence:
- Toolchain local:
  - `rustc 1.94.1 (e408947bf 2026-03-25)`
  - `cargo 1.94.1`
- O repositório está em `Z:` e o volume é `ReFS` (`FileSystem = ReFS`, label `SODA_Forge`).
- Reproduzido em `Z:\genesis_mc\src-tauri` com target padrão:
  - `cargo test --features tauri-app --bin n0_daemon_watcher` -> `0` com warning
  - warning: `error finalizing incremental compilation session directory ... Access is denied. (os error 5)`
- Também houve um `rustc` ICE anterior durante `cargo run` com incremental ativo, encerrando com `no entry found for key` dentro de `rustc_metadata::rmeta::encoder`.
- Com mitigação local:
  - `CARGO_INCREMENTAL=0 cargo check --features tauri-app --bin n0_daemon_watcher` -> `0`
  - `cargo test` usando `CARGO_TARGET_DIR=%TEMP%\genesis_mc_target_ntfs` -> `0` sem warning

External Correlation:
- Há issue aberta no Rust sobre regressão no Windows/ReFS com exatamente `error finalizing incremental compilation session directory ... Access is denied. (os error 5)`: `rust-lang/rust#151181`.
- O texto da issue indica reprodução em ReFS/Dev Drive desde Rust `1.90.0+`, compatível com este ambiente e com o comportamento observado.

Conclusion:
- O ruído não é apenas cosmético. Ele nasce de uma combinação real entre `incremental compilation` e `ReFS` no Windows.
- No melhor caso, vira warning intermitente sem bloquear build.
- No pior caso, pode escalar para falhas de compilação esporádicas/ICE do `rustc`, como já ocorreu nesta sessão.
- O código do projeto não é a causa primária; a evidência aponta para ambiente/toolchain.

Recommended Mitigations:
1. Para validações críticas, usar `CARGO_INCREMENTAL=0`.
2. Para manter incremental, mover `CARGO_TARGET_DIR` para `%TEMP%`/NTFS.
3. Evitar builds paralelos compartilhando o mesmo `target` em `Z:` durante sessões longas.
4. Monitorar updates do Rust para o bug `#151181`; avaliar downgrade controlado para uma versão sem a regressão se o ruído piorar.

Decision:
- Mitigação adotada: manter o repositório e o `target` em `ReFS` e usar `CARGO_INCREMENTAL=0`.
- O launcher principal já aplica essa mitigação antes do `cargo run`.
- Não foi adotado redirecionamento do `target` para `NTFS`, por decisão explícita de performance.
