# Design Specification — MARCO 5.14.0 (Drift Sentinel)

```mermaid
graph TD
    A[Tokio Timer 3600s] --> B{is_internet_active UDP}
    B -- Offline --> C[Ociosidade Tática / Continue]
    B -- Online --> D[STATE_DB_TX: GetRepositoriesForDriftCheck]
    D --> E[SQLite Query JOIN repositorios & repo_heuristics]
    E --> F[List of RepoDriftCandidate]
    F --> G[reqwest HTTP GET GitHub Releases/Tags]
    G --> H{repo_version != online_version?}
    H -- Yes --> I[STATE_DB_TX: UpdateRepositoryDrift]
    I --> J[SQLite Update: PENDENTE_FASE_0 & PENDENTE & Timestamp]
    H -- No --> K[No Op]
```

## Hardware-Agnostic Statement
Solution operates asynchronously in host network stack without consuming GPU/VRAM resources, adhering strictly to FinOps and local-first architecture.
