# MARCO 5.7.0 — Chyros Daemon, Consolidação Episódica e Decaimento Langevin

## 1. Contexto & Objetivos

Em conformidade com o SODA Canon V6, ADR-030 (Higiene Bare-Metal), ADR-032 (Governança da Memória Tri-Partite) e ADR-027 (Termodinâmica de CPU/GPU):
- **ChyrosDaemon (AutoDream)**: Daemon assíncrono que monitora a ociosidade do sistema (quando não há requisições ativas do usuário) e realiza a consolidação cognitiva episódica e o decaimento orgânico de memórias em segundo plano.
- **Interrupção de Segurança (100% P-Cores)**: Ao menor sinal de interação síncrona do usuário, o daemon congela e cancela imediatamente qualquer execução intensiva de CPU.
- **Roteamento de Hardware Zero-GPU**: Processamento do Gemma E2B (Tier 0.5) executado estritamente na CPU (AVX2) via `LlamaCpp4LogitEngine` sem alocar pesos na RTX 2060m.
- **Decaimento de Langevin na Bola de Poincaré**: Algoritmo de desvio estocástico PGD ($x_{t+1} = \text{proj\_Poincare}(x_t - \eta \nabla V(x_t) + \sqrt{2D\Delta t}\xi_t)$) onde memórias frias evolutivas sofrem drift até o limiar de evicção $\|x\| \ge 0.95$.
- **Materialized Memory View (MMV)**: Snapshot consolidado em RAM alinhado em múltiplos de 64 tokens para atingir 95%+ Prefix Cache Hit Rate.

## 2. Arquitetura Orchestrator-Worker

```mermaid
graph TD
    UserActivity[Interação do Usuário / API Síncrona] -->|Interrupção Incondicional| ActivityTracker[ActivityTracker: AtomicU64]
    ChyrosDaemon[ChyrosDaemon Loop 60s] -->|Verifica Idle Threshold| ActivityTracker
    ChyrosDaemon -->|Se Idle| AutoDream[Fase 1: AutoDream Cycle]
    AutoDream -->|Langevin Decay| Poincaré[Poda de Langevin na Bola de Poincaré]
    Poincaré -->|Norma >= 0.95| Eviction[Marca SUPERSEDED / Evicção]
    AutoDream -->|L0 Consolidate| GemmaCPU[Gemma E2B / LlamaCpp4LogitEngine na CPU]
    GemmaCPU -->|Resolução Socrática de Contradições| SQLite[souls_state.db: souls_memory_nodes & raw_events_l0]
    SQLite -->|Snapshot Alinhado 64-Tokens| MMV[Materialized Memory View (RAM)]
```

## 3. Agnosticismo de Hardware

- 100% CPU Bare-Metal (AVX2/Tokio).
- Isenção total de alocações na GPU dGPU (RTX 2060m).
- Estruturas de dados em memória serializáveis e transmutáveis para C/Rust bare-metal.

## 4. Definição de Done (DoD GREEN)

- `test_chyros_daemon_idle_trigger`
- `test_langevin_decay_convergence`
- `test_jit_factual_consolidation`
- `test_mmv_prefix_cache_rate`
- Clippy 100% limpo com zero warnings (`cargo clippy -- -D warnings`).
