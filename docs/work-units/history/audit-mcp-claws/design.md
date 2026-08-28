# Design Document — Operação Auto-Raio-X: Auditoria Comportamental e Estresse das 50 Garras MCP

## 1. Contexto e Objetivos

Conforme as **ADR-001** (Core Stack Tokio/Rust), **ADR-003** (Isolamento de Stdio e Cerca do Stderr), **ADR-010** (Pipeline SDD-TDD), **ADR-025** (Qualidade 100/100), **ADR-041** (Nomenclatura Soberana `souls_mcp`) e **ADR-043** (Observabilidade Cognitiva), esta Work Unit implanta o harness clínico automatizado `soda_mcp_tester_cli` para auditar a integridade, pureza de canal, maturidade funcional e latência em microssegundos de todas as 50 garras MCP expostas pelo servidor `souls_mcp_server`.

### Metas Centrais:
1. **Harness Bare-Metal em Rust (`soda_mcp_tester_cli`):** Executar chamadas JSON-RPC 2.0 reais via pipes stdio simulando a Cursor/Trae IDE.
2. **Auditoria Exhaustiva das 50 Garras:** Testar cada ferramenta registrada em `tools::list_tools()` / `router::handle_tool_call` com payloads válidos.
3. **Inspeção de Pureza Stdio (ADR-003):** Checar ausência total de escapes ANSI, warnings de compilador e logs fora do protocolo JSON-RPC.
4. **Classificação Forense de Maturidade:** Categorizar cada ferramenta como `[LIVE_PRODUCTION]`, `[STUB_MOCK]` ou `[BROKEN_ERROR]`.
5. **Geração Automatizada do Laudo Clínico:** Escrever o relatório forense detalhado em `.souls_scratchpad/reports/mcp_claws_clinical_audit.md`.

---

## 2. Diagrama Arquitetural Orchestrator-Worker

```mermaid
flowchart TD
    subgraph Test Harness ["soda_mcp_tester_cli"]
        Runner[Audit Test Runner]
        PayloadGen[Payload & Schema Generator]
        StdioClient[Async Stdio JSON-RPC Client]
        LatencyTimer[Instant::now Timer (us)]
        HygieneValidator[ANSI & Stdio Inspector (ADR-003)]
        Classifier[Maturity Classifier (LIVE / STUB / BROKEN)]
        ReportGen[Markdown Forensics Generator]
    end

    subgraph Target Subprocess ["souls_mcp_server (stdio)"]
        ServerStdin[BufReader Stdin]
        Reactor[JSON-RPC Dispatcher]
        Router[router::handle_tool_call]
        DBWorkers[FrankenSQLite / LanceDB Workers]
        ServerStdout[BufWriter Stdout]
    end

    subgraph Output Forensics
        ReportFile[".souls_scratchpad/reports/mcp_claws_clinical_audit.md"]
        ClippyLog[".souls_scratchpad/logs/misc/clippy_mcp_audit.log"]
    end

    Runner --> PayloadGen
    PayloadGen --> StdioClient
    StdioClient -->|JSON-RPC Request Frame| ServerStdin
    ServerStdin --> Reactor
    Reactor --> Router
    Router --> DBWorkers
    Reactor --> ServerStdout
    ServerStdout -->|JSON-RPC Response Frame| StdioClient
    StdioClient --> LatencyTimer
    StdioClient --> HygieneValidator
    HygieneValidator --> Classifier
    Classifier --> ReportGen
    ReportGen --> ReportFile
```

---

## 3. Matriz de Classificação de Maturidade

| Categoria | Critério Clínico |
| :--- | :--- |
| **`LIVE_PRODUCTION`** | Executa lógica real, toca disco, AST Tree-Sitter, bancos FrankenSQLite / LanceDB ou serviços nativos sem mensagens de fallback. |
| **`STUB_MOCK`** | Retorna erros declarados de stub (`not_implemented_yet`, `todo`, `stub_sandbox_audit_pending`) ou payloads estáticos em RAM. |
| **`BROKEN_ERROR`** | Pânicos, violações de canal stdio (bytes ANSI / lixo), timeouts ou erros de desserialização não interceptados. |
