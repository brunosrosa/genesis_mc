# SODA Current State - SOULS V4 Architectural State & ADR Registry

Last Updated: 2026-08-01

## Status Geral dos ADRs Constitucionais

| ID | Título | Status | Épico | Resumo / Atualizações V4 |
|---|---|---|---|---|
| ADR-001 | Core Stack Restrita | Ativo_Inegociavel | Arquitetura | Rust backend, Svelte 5 frontend, Zero-VDOM. |
| ADR-002 | Sandboxing Híbrido | Ativo_Inegociavel | Segurança | Landlock / AppContainer isolamento de subprocessos. |
| ADR-003 | Zero-Copy IPC | Ativo_Inegociavel | IPC | iceoryx2 data plane, stdio isolation no MCP, **Zero-allocation slice parsing (>1MB payloads)**. |
| ADR-004 | Tríade de Memória | Ativo_Inegociavel | Memória | SQLite WAL Append-Only, LanceDB, LadybugDB, **zstd compression**, **Memória Neuro-Sintética (MNS)**, **Gotcha/Wakeup**. |
| ADR-005 | RAG Temporal | Ativo_Inegociavel | Memória | Extração de datas na CPU, BTREE no LanceDB, tags STABLE/EVOLVING, **Diários de Bordo**, **Briefing AAAK Gotcha/Wakeup**. |
| ADR-011 | Governança HITL | Ativo_Inegociavel | Infraestrutura | Protocolo BMAD, **Gerenciador file_locker.rs**, **Mutex OnceLock+DashMap**, **Limpeza de Arc::strong_count==1**, **Atomic-write-file**. |
| ADR-027 | Motor de Inferência Híbrido & VRAM | Ativo_Inegociavel | Cognição | RTX 2060m limit, **EngineCascade**, **TopologyFeatures O(1) via mmap**, **Worker FFI C++ (souls_vanguard_worker.exe)**, **minijinja zero-alloc**. |
| ADR-037 | Gestão Dinâmica de Contexto CCR | Ativo_Inegociavel | Infraestrutura | Algoritmo Headroom em Rust, DashMap Host RAM, **souls_compress_memory**, **souls_dedup**, **souls_fill**. |
| ADR-038 | Execução Elástica & Compressão de Logs | Proposed | Infraestrutura | **Isolamento de Stdio via tokio::process::Command**, **Pattern Log Compression (90% pruning)**. |
| ADR-040 | Migração State DB v2 + Disjuntor Cognitivo | Ativo_Inegociavel | Cognição | **Marco 3.5 ATIVO**: souls_graph (9 tools mem_*) + souls_thinking (core_think, disjuntor 5→7 HITL). Tabela `observations` normalizada + FTS5 + triggers. PRAGMA user_version=2. |

---

## Próximos Passos & Validação de Compilação
- Validação contínua de integridade via suíte de testes Rust: `cargo test --lib --features "tauri-app,gateway_ccr,llama_backend"`
