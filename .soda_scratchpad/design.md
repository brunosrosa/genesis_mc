---
title: "Design de Higiene (Dualidade Fábrica vs Produto)"
status: "ativo"
---

## Invariantes

- O núcleo do ETL (Fase 1.5) é soberano: mudanças apenas para correções de portabilidade/higiene.
- “Fábrica” pode iniciar ferramentas auxiliares, mas não pode impedir build/test em Linux/CI.
- “Produto” permanece especificado como Rust/Tokio + Svelte 5 + Vite + Tauri v2, com UI passiva.

## Separação Operacional

### Frontend

- Terreno preparado para Svelte 5; React removido.
- A build do frontend precisa permanecer estável para `cargo tauri dev/build` (mesmo com UI hibernada, o scaffold deve compilar).

### Tauri Entry-point e IPC

- O `invoke_handler` deve registrar comandos essenciais (ex.: `genesis_ping`) sem interferir no bootstrap do AgentGateway (quando habilitado).
- O bootstrap do AgentGateway deve residir em `setup()` e ser tolerante a ambiente (não deve quebrar build/test).

### FinOps Router / Testes

- Nenhum teste deve depender de paths absolutos de máquina.
- Smoke tests que dependem de recursos locais devem localizar assets dentro do repositório via `env!("CARGO_MANIFEST_DIR")` (ou fazer skip determinístico).

