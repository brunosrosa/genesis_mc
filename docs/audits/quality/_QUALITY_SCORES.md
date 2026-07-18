# Auditoria Qualitativa dos Blobs (Fase 0 Harvester)

**Gerado em:** 2026-07-18T17:16:09

**Pares (repo, blob) auditados:** 33

**Referência:** spec-040 / ADR-031 §4 (anatomia dos 11 blobs)


## 1. Score por (repo, blob)

| repo_id | 01_promessa_readme | 02_dependency_manifest | 03_test_intent | 04_repo_outline | 05_architecture_map | 06_unsafe_hotspots | 07_ops_blueprint | 08_health_report | 09_community_meta | 10_soda_canon_context | 11_ux_contracts | avg |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `bytecodealliance/wasmtime` | 82 | 76 | 78 | 69 | 68 | 81 | 86 | 50 | 61 | 88 | 58 | **72.5** |
| `mendableai/firecrawl` | 86 | 83 | 78 | 66 | 89 | 81 | 78 | 50 | 63 | 88 | 58 | **74.6** |
| `tldraw/tldraw` | 79 | 73 | 68 | 66 | 68 | 81 | 86 | 59 | 63 | 88 | 58 | **71.8** |

## 2. Agregado por artifact_type (sistêmico)

| blob | média | std | min | max | n |
|---|---:|---:|---:|---:|---:|
| blob_01_promessa_readme | **82.6** | 2.7 | 79 | 86 | 3 |
| blob_02_dependency_manifest | **77.3** | 4.2 | 73 | 83 | 3 |
| blob_03_test_intent | **74.7** | 4.7 | 68 | 78 | 3 |
| blob_04_repo_outline | **67.0** | 1.4 | 66 | 69 | 3 |
| blob_05_architecture_map | **75.0** | 9.9 | 68 | 89 | 3 |
| blob_06_unsafe_hotspots | **81.3** | 0.0 | 81 | 81 | 3 |
| blob_07_ops_blueprint | **83.3** | 3.8 | 78 | 86 | 3 |
| blob_08_health_report | **53.1** | 4.3 | 50 | 59 | 3 |
| blob_09_community_meta | **62.3** | 0.9 | 61 | 63 | 3 |
| blob_10_soda_canon_context | **88.0** | 0.0 | 88 | 88 | 3 |
| blob_11_ux_contracts | **58.0** | 0.0 | 58 | 58 | 3 |

## 3. Top 10 piores casos (score < 50)

| repo_id | blob | size | score | detail |
|---|---|---:|---:|---|
| (nenhum abaixo de 50) | | | | |

## 4. Top 10 melhores casos (score ≥ 80)

| repo_id | blob | size | score |
|---|---|---:|---:|
| `mendableai/firecrawl` | blob_05_architecture_map | 5405 | **89.0** |
| `bytecodealliance/wasmtime` | blob_10_soda_canon_context | 5803 | **88.0** |
| `mendableai/firecrawl` | blob_10_soda_canon_context | 5803 | **88.0** |
| `tldraw/tldraw` | blob_10_soda_canon_context | 5803 | **88.0** |
| `bytecodealliance/wasmtime` | blob_07_ops_blueprint | 97382 | **86.0** |
| `mendableai/firecrawl` | blob_01_promessa_readme | 24098 | **86.0** |
| `tldraw/tldraw` | blob_07_ops_blueprint | 93269 | **86.0** |
| `mendableai/firecrawl` | blob_02_dependency_manifest | 6340 | **83.0** |
| `bytecodealliance/wasmtime` | blob_01_promessa_readme | 7673 | **82.33** |
| `bytecodealliance/wasmtime` | blob_06_unsafe_hotspots | 869 | **81.33** |

## 5. Resumo executivo

- **Blob mais fraco (sistêmico):** `blob_08_health_report` com média 53.1
- **Blob mais forte (sistêmico):** `blob_10_soda_canon_context` com média 88.0
- **Violações de Lei IV (ADR-031):** 2 / 33
- **Violações de rebrand (`genesis_mc` residual):** 0 / 33
- **Casos com slop (TODO/FIXME/PLACEHOLDER):** 0 / 33
