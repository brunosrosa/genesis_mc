---
spec: feat-l3-hippocampus
version: 1.0
status: Active
branch: feat/l3-hippocampus
author: souls-rust-expert
date: 2026-08-16
red_line: PROIBIDO o uso de mocks em RAM ordinária ou de simular buscas cosseno. PROIBIDO alocação de tensores ou índices na VRAM da RTX 2060m (0 MB VRAM absoluto). PROIBIDO travamento do event loop Tokio (latência sub-5ms em CPU AVX2).
acao_de_canibalizacao: Implantação física da Memória Semântica Real L3 (LanceDB mmap em NVMe), Reator de Fusão Híbrida RRF acelerado via CPU AVX2 (unindo FTS5 do FrankenSQLite com similaridade vetorial LanceDB) e Firewall Ontológico LadybugDB (DashMap em RAM Host com travessia BFS anti-RAG poisoning) para a ferramenta souls_semantic_search.
---

# SDD Design: MARCO VI — Operação Âncora do Hipocampo (LanceDB mmap, RRF AVX2 e LadybugDB)

## 1. Contexto & Objetivos

A presente Work Unit implanta o motor físico de busca híbrida real para a ferramenta `souls_semantic_search` e o subsistema de memória L3/L2/L1 do SOULS MC, em conformidade com:
- **ADR-001** (Core Stack: Tokio Bare-Metal Rust)
- **ADR-003** (Isolamento de Stdio / Canais Protegidos)
- **ADR-005** (RAG Temporal: Bifurcação temporal, RRF híbrido e bypass de falso negativo)
- **ADR-010** (Pipeline SDD-TDD Rigoroso)
- **ADR-025** (Qualidade 100/100, zero warnings)
- **ADR-027** (Termodinâmica de VRAM: 0 MB de oscilação na RTX 2060m)
- **ADR-030** (Higiene de Dependências: windows-sys, isolamento de crates)
- **ADR-041** (Nomenclatura Soberana `souls_mcp` e Zero-Brand)

## 2. Linhas Vermelhas (Invioláveis)

| # | Regra | Justificativa |
|---|-------|---------------|
| R1 | Zero VRAM Footprint | LanceDB opera 100% via `mmap` em NVMe e Host RAM. Delta de VRAM na RTX 2060m = 0 MB. |
| R2 | Cura do Colapso IVF-PQ | Quando o filtro escalar temporal for restritivo (< 1000 registros), força `bypass_vector_index()` com kNN exato em RAM para erradicar amnésia. |
| R3 | Fusão Híbrida RRF Sub-5ms | Reator RRF na CPU do host com vetorização SIMD AVX2 unificando candidatos léxicos (SQLite FTS5) e vetoriais (LanceDB). |
| R4 | Firewall Ontológico LadybugDB | `DashMap` em RAM Host com travessia BFS em grafo causal. Bloqueio e banimento sumário de qualquer chunk com contradição a ADRs `STABLE`. |
| R5 | Zero Tokio Event-Loop Stall | Execuções pesadas encapsuladas em workers Tokio bloqueantes e MPSC assíncronos. |

## 3. Topologia Orchestrator-Worker & Agnosticismo de Hardware

```mermaid
flowchart TD
    CLIENT[MCP Client / Frontend Tauri IPC] -->|souls_semantic_search| ROUTER[MCP Server Router / system.rs]
    
    subgraph ORCHESTRATOR [Orchestrator: Tokio Async Runtime]
        ROUTER --> PAR_EXEC[tokio::join / Spawn Paralelo]
        PAR_EXEC -->|Query Léxica| FTS_RET[FtsRetriever: SQLite FTS5 BM25]
        PAR_EXEC -->|Query Vetorial| VEC_RET[VectorRetriever: LanceDB mmap]
        
        VEC_RET -->|Barreira Escalar < 1000| BYPASS[bypass_vector_index kNN Exato]
        VEC_RET -->|Massa > 1000| IVF[IVF-PQ Index Search]
    end
    
    subgraph STORAGE_TRIAD [Tríade de Memória L1/L2/L3]
        L1_GRAPH[(L1: LadybugDB DashMap RAM)]
        L2_SQLITE[(L2: FrankenSQLite souls_state.db FTS5)]
        L3_LANCE[(L3: LanceDB semantic_memories NVMe mmap)]
        
        FTS_RET <--> L2_SQLITE
        VEC_RET <--> L3_LANCE
    end
    
    subgraph CPU_REACTOR [Reator de Fusão CPU AVX2]
        FTS_RES[Lexical Matches] --> RRF_ENG[RRF Fusion Engine: k=60 + SIMD AVX2]
        VEC_RES[Vector Matches] --> RRF_ENG
        TOMB[Tombstones Invalidation] --> RRF_ENG
        RRF_ENG --> FUSED[UnifiedMatch Ranked List]
    end
    
    PAR_EXEC --> FTS_RES
    PAR_EXEC --> VEC_RES
    
    subgraph LADYBUG_SENTINEL [Firewall Ontológico Anti-RAG Poisoning]
        FUSED --> BFS_WALK[BFS Graph Traversal: Max Depth 4]
        BFS_WALK <--> L1_GRAPH
        BFS_WALK -->|Detecção de Conflito com STABLE| PURGE[Expurgo de Chunk & Alerta Epistêmico]
        BFS_WALK -->|Aprovado| CLEAN[Payload Sanitizado]
    end
    
    CLEAN --> JSON_RESP[JSON-RPC Result Ordenado por RRF Score]
```

## 4. Agnosticismo de Hardware
- **Piso de Validação (Treino de Gravidade):** Intel Core i9 + NVIDIA RTX 2060m (6GB VRAM).
- **Isolamento Total:** A dGPU não é tocada durante o fluxo RAG/vetorial (0 MB VRAM).
- **Vetorização CPU:** Instruções AVX2 para soma e normalização de scores RRF. Transmutável para ARM NEON / NPU sem reescrever a arquitetura.

## 5. Especificação dos Contratos Físicos

### 5.1. LanceDB (L3 Vector Memory)
- **Path Canônico:** `Z:\souls_mc\.souls_data\semantic_memories`
- **Schema Arrow:**
  - `id`: Utf8
  - `text_content`: Utf8
  - `embedding`: FixedSizeList(384, Float32)
  - `temporal_stability`: Utf8 ("STABLE" | "EVOLVING")
  - `valid_from`: Int64 (Unix Epoch)
  - `valid_to`: Int64 (Nullable Unix Epoch)
- **Mecanismo Zero-VRAM:** Abertura serverless local com `lancedb::connect()`, leitura orientada a Arrow RecordBatches mapeados em Host RAM.

### 5.2. Reator de Fusão RRF (Reciprocal Rank Fusion)
- **Constante padrão:** $k = 60$.
- **Fórmula:**
  $$RRF(d) = \sum_{m \in \{lex, vec\}} \frac{1}{k + r_m(d)} + Bonus_{exact}$$
- **Bonus de Termo Exato:** $+10.0$ para correspondência exata com termos canônicos e palavras-chave rígidas (`ADR-xxx`, `windows-sys`, etc.).
- **Latência Alvo:** $< 5$ ms para unificar 500 candidatos.

### 5.3. Firewall Ontológico LadybugDB
- **Grafo em Memória:** `Arc<DashMap<String, OntologicalNode>>` e `Arc<DashMap<String, Vec<OntologicalEdge>>>`.
- **Validação Fail-Closed:** Travessia BFS a partir de entidades no chunk. Se colidir com nós `STABLE` com `banned_patterns`, veta o chunk e impede contaminação de contexto.
