# DB_STATE_REPORT — soda_heuristic_vault.db

- Generated at (UTC): 2026-05-25T23:56:24Z
- Database path: c:\Users\rosas\Dev_Projects\genesis_mc\.soda_data\soda_heuristic_vault.db
- SQLite version: 3.49.1
- PRAGMA user_version: 0
- PRAGMA foreign_keys: 0

## Inventory

- Objects: table=13, view=3, index=1

## TABLES

### artefatos_brutos

```sql
CREATE TABLE artefatos_brutos (
    artifact_id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id TEXT NOT NULL REFERENCES repositorios(project_name),
    payload_blob TEXT NOT NULL,
    timestamp_extracao INTEGER NOT NULL,
    artifact_type TEXT NOT NULL
)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | artifact_id | INTEGER | 0 |  | 1 |
| 1 | repo_id | TEXT | 1 |  | 0 |
| 2 | payload_blob | TEXT | 1 |  | 0 |
| 3 | timestamp_extracao | INTEGER | 1 |  | 0 |
| 4 | artifact_type | TEXT | 1 |  | 0 |

**Foreign Keys**
| id | seq | table | from | to | on_update | on_delete | match |
|---:|---:|---|---|---|---|---|---|
| 0 | 0 | repositorios | repo_id | project_name | NO ACTION | NO ACTION | NONE |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| idx_artefatos_repo_tipo | 1 | c | 0 | repo_id, artifact_type |

### artefatos_destilados

```sql
CREATE TABLE artefatos_destilados (
            distilled_id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id TEXT NOT NULL,
            essence_name TEXT NOT NULL,
            routing_zone TEXT NOT NULL,
            token_count INTEGER NOT NULL,
            destination TEXT NOT NULL,
            payload_essence TEXT NOT NULL,
            timestamp_destilacao INTEGER NOT NULL,
            UNIQUE(repo_id, essence_name)
        )
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | distilled_id | INTEGER | 0 |  | 1 |
| 1 | repo_id | TEXT | 1 |  | 0 |
| 2 | essence_name | TEXT | 1 |  | 0 |
| 3 | routing_zone | TEXT | 1 |  | 0 |
| 4 | token_count | INTEGER | 1 |  | 0 |
| 5 | destination | TEXT | 1 |  | 0 |
| 6 | payload_essence | TEXT | 1 |  | 0 |
| 7 | timestamp_destilacao | INTEGER | 1 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_artefatos_destilados_1 | 1 | u | 0 | repo_id, essence_name |

### debates_enxame

```sql
CREATE TABLE debates_enxame (
    debate_id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id TEXT NOT NULL REFERENCES repositorios(project_name),
    contexto_oraculo_soda TEXT NOT NULL,
    lente_a_output_bruto TEXT NOT NULL,
    lente_a_produto_ux TEXT NOT NULL,
    lente_b_output_bruto TEXT NOT NULL,
    lente_b_arquitetura TEXT NOT NULL,
    lente_c_output_bruto TEXT NOT NULL,
    lente_c_operacao TEXT NOT NULL,
    timestamp_inicio_debate INTEGER NOT NULL,
    timestamp_fim_fase_2 INTEGER NOT NULL
, lens_a_json TEXT NOT NULL DEFAULT '', lens_b_json TEXT NOT NULL DEFAULT '', lens_c_json TEXT NOT NULL DEFAULT '', phase_status TEXT NOT NULL DEFAULT 'PENDING', model_used TEXT NOT NULL DEFAULT '{}')
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | debate_id | INTEGER | 0 |  | 1 |
| 1 | repo_id | TEXT | 1 |  | 0 |
| 2 | contexto_oraculo_soda | TEXT | 1 |  | 0 |
| 3 | lente_a_output_bruto | TEXT | 1 |  | 0 |
| 4 | lente_a_produto_ux | TEXT | 1 |  | 0 |
| 5 | lente_b_output_bruto | TEXT | 1 |  | 0 |
| 6 | lente_b_arquitetura | TEXT | 1 |  | 0 |
| 7 | lente_c_output_bruto | TEXT | 1 |  | 0 |
| 8 | lente_c_operacao | TEXT | 1 |  | 0 |
| 9 | timestamp_inicio_debate | INTEGER | 1 |  | 0 |
| 10 | timestamp_fim_fase_2 | INTEGER | 1 |  | 0 |
| 11 | lens_a_json | TEXT | 1 | '' | 0 |
| 12 | lens_b_json | TEXT | 1 | '' | 0 |
| 13 | lens_c_json | TEXT | 1 | '' | 0 |
| 14 | phase_status | TEXT | 1 | 'PENDING' | 0 |
| 15 | model_used | TEXT | 1 | '{}' | 0 |

**Foreign Keys**
| id | seq | table | from | to | on_update | on_delete | match |
|---:|---:|---|---|---|---|---|---|
| 0 | 0 | repositorios | repo_id | project_name | NO ACTION | NO ACTION | NONE |

### deep_components

```sql
CREATE TABLE deep_components (
    component_id TEXT NOT NULL PRIMARY KEY,
    solution_id TEXT NOT NULL REFERENCES repositorios(project_name),
    component_name TEXT NOT NULL,
    component_group TEXT NOT NULL,
    component_real_role TEXT NOT NULL,
    component_deep_pattern TEXT NOT NULL,
    component_capability_nature TEXT NOT NULL,
    component_architectural_topology TEXT NOT NULL,
    absorption_classification TEXT NOT NULL,
    absorption_mode TEXT NOT NULL,
    absorption_reasoning TEXT NOT NULL,
    requires_reimplementation TEXT NOT NULL,
    do_not_cross_line TEXT NOT NULL,
    component_extractability TEXT NOT NULL,
    component_bare_metal_fit TEXT NOT NULL,
    component_runtime_sovereignty TEXT NOT NULL,
    component_operability TEXT NOT NULL,
    component_creep_risk TEXT NOT NULL,
    ecosystem_position TEXT NOT NULL,
    component_summary_sentence TEXT NOT NULL,
    component_verdict TEXT NOT NULL,
    score_component_value INTEGER NOT NULL,
    score_component_reusability INTEGER NOT NULL,
    score_component_fit INTEGER NOT NULL,
    score_component_risk INTEGER NOT NULL,
    score_component_priority INTEGER NOT NULL,
    score_component_extractability INTEGER NOT NULL,
    score_component_bare_metal_fit INTEGER NOT NULL,
    score_component_runtime_sovereignty INTEGER NOT NULL,
    score_component_logic_value INTEGER NOT NULL,
    score_component_ethics_safety INTEGER NOT NULL,
    score_component_operability INTEGER NOT NULL,
    score_component_adoptability INTEGER NOT NULL,
    score_component_sustainability INTEGER NOT NULL,
    component_transplantable_core TEXT NOT NULL,
    component_core_value_location TEXT NOT NULL,
    component_logic_value TEXT NOT NULL,
    component_math_or_heuristic_value TEXT NOT NULL,
    component_data_model_value TEXT NOT NULL,
    component_state_model_value TEXT NOT NULL,
    component_relation_model_value TEXT NOT NULL,
    component_regulatory_risk TEXT NOT NULL
)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | component_id | TEXT | 1 |  | 1 |
| 1 | solution_id | TEXT | 1 |  | 0 |
| 2 | component_name | TEXT | 1 |  | 0 |
| 3 | component_group | TEXT | 1 |  | 0 |
| 4 | component_real_role | TEXT | 1 |  | 0 |
| 5 | component_deep_pattern | TEXT | 1 |  | 0 |
| 6 | component_capability_nature | TEXT | 1 |  | 0 |
| 7 | component_architectural_topology | TEXT | 1 |  | 0 |
| 8 | absorption_classification | TEXT | 1 |  | 0 |
| 9 | absorption_mode | TEXT | 1 |  | 0 |
| 10 | absorption_reasoning | TEXT | 1 |  | 0 |
| 11 | requires_reimplementation | TEXT | 1 |  | 0 |
| 12 | do_not_cross_line | TEXT | 1 |  | 0 |
| 13 | component_extractability | TEXT | 1 |  | 0 |
| 14 | component_bare_metal_fit | TEXT | 1 |  | 0 |
| 15 | component_runtime_sovereignty | TEXT | 1 |  | 0 |
| 16 | component_operability | TEXT | 1 |  | 0 |
| 17 | component_creep_risk | TEXT | 1 |  | 0 |
| 18 | ecosystem_position | TEXT | 1 |  | 0 |
| 19 | component_summary_sentence | TEXT | 1 |  | 0 |
| 20 | component_verdict | TEXT | 1 |  | 0 |
| 21 | score_component_value | INTEGER | 1 |  | 0 |
| 22 | score_component_reusability | INTEGER | 1 |  | 0 |
| 23 | score_component_fit | INTEGER | 1 |  | 0 |
| 24 | score_component_risk | INTEGER | 1 |  | 0 |
| 25 | score_component_priority | INTEGER | 1 |  | 0 |
| 26 | score_component_extractability | INTEGER | 1 |  | 0 |
| 27 | score_component_bare_metal_fit | INTEGER | 1 |  | 0 |
| 28 | score_component_runtime_sovereignty | INTEGER | 1 |  | 0 |
| 29 | score_component_logic_value | INTEGER | 1 |  | 0 |
| 30 | score_component_ethics_safety | INTEGER | 1 |  | 0 |
| 31 | score_component_operability | INTEGER | 1 |  | 0 |
| 32 | score_component_adoptability | INTEGER | 1 |  | 0 |
| 33 | score_component_sustainability | INTEGER | 1 |  | 0 |
| 34 | component_transplantable_core | TEXT | 1 |  | 0 |
| 35 | component_core_value_location | TEXT | 1 |  | 0 |
| 36 | component_logic_value | TEXT | 1 |  | 0 |
| 37 | component_math_or_heuristic_value | TEXT | 1 |  | 0 |
| 38 | component_data_model_value | TEXT | 1 |  | 0 |
| 39 | component_state_model_value | TEXT | 1 |  | 0 |
| 40 | component_relation_model_value | TEXT | 1 |  | 0 |
| 41 | component_regulatory_risk | TEXT | 1 |  | 0 |

**Foreign Keys**
| id | seq | table | from | to | on_update | on_delete | match |
|---:|---:|---|---|---|---|---|---|
| 0 | 0 | repositorios | solution_id | project_name | NO ACTION | NO ACTION | NONE |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_deep_components_1 | 1 | pk | 0 | component_id |

### etl_errors

```sql
CREATE TABLE etl_errors (
    error_id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_name TEXT NOT NULL REFERENCES repositorios(project_name),
    failed_phase TEXT NOT NULL,
    stacktrace_erro TEXT NOT NULL,
    timestamp_erro INTEGER NOT NULL
)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | error_id | INTEGER | 0 |  | 1 |
| 1 | project_name | TEXT | 1 |  | 0 |
| 2 | failed_phase | TEXT | 1 |  | 0 |
| 3 | stacktrace_erro | TEXT | 1 |  | 0 |
| 4 | timestamp_erro | INTEGER | 1 |  | 0 |

**Foreign Keys**
| id | seq | table | from | to | on_update | on_delete | match |
|---:|---:|---|---|---|---|---|---|
| 0 | 0 | repositorios | project_name | project_name | NO ACTION | NO ACTION | NONE |

### etl_run_log

```sql
CREATE TABLE etl_run_log (
    run_id TEXT NOT NULL PRIMARY KEY,
    lote_id TEXT NOT NULL,
    timestamp_inicio INTEGER NOT NULL,
    timestamp_fim INTEGER,
    status_lote TEXT NOT NULL,
    cached_input_tokens INTEGER NOT NULL,
    total_input_tokens INTEGER NOT NULL,
    total_output_tokens INTEGER NOT NULL,
    total_cost_usd REAL NOT NULL,
    primary_model_routed TEXT NOT NULL,
    total_repos_processed INTEGER NOT NULL,
    total_short_circuits INTEGER NOT NULL,
    execution_latency_ms INTEGER NOT NULL
)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | run_id | TEXT | 1 |  | 1 |
| 1 | lote_id | TEXT | 1 |  | 0 |
| 2 | timestamp_inicio | INTEGER | 1 |  | 0 |
| 3 | timestamp_fim | INTEGER | 0 |  | 0 |
| 4 | status_lote | TEXT | 1 |  | 0 |
| 5 | cached_input_tokens | INTEGER | 1 |  | 0 |
| 6 | total_input_tokens | INTEGER | 1 |  | 0 |
| 7 | total_output_tokens | INTEGER | 1 |  | 0 |
| 8 | total_cost_usd | REAL | 1 |  | 0 |
| 9 | primary_model_routed | TEXT | 1 |  | 0 |
| 10 | total_repos_processed | INTEGER | 1 |  | 0 |
| 11 | total_short_circuits | INTEGER | 1 |  | 0 |
| 12 | execution_latency_ms | INTEGER | 1 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_etl_run_log_1 | 1 | pk | 0 | run_id |

### kanban_tasks

```sql
CREATE TABLE kanban_tasks (
    task_id TEXT NOT NULL PRIMARY KEY,
    target_repo_id TEXT NOT NULL REFERENCES repositorios(project_name),
    issue_ref TEXT,
    task_title TEXT NOT NULL,
    status TEXT NOT NULL,
    associated_pr TEXT,
    last_updated INTEGER NOT NULL
)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | task_id | TEXT | 1 |  | 1 |
| 1 | target_repo_id | TEXT | 1 |  | 0 |
| 2 | issue_ref | TEXT | 0 |  | 0 |
| 3 | task_title | TEXT | 1 |  | 0 |
| 4 | status | TEXT | 1 |  | 0 |
| 5 | associated_pr | TEXT | 0 |  | 0 |
| 6 | last_updated | INTEGER | 1 |  | 0 |

**Foreign Keys**
| id | seq | table | from | to | on_update | on_delete | match |
|---:|---:|---|---|---|---|---|---|
| 0 | 0 | repositorios | target_repo_id | project_name | NO ACTION | NO ACTION | NONE |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_kanban_tasks_1 | 1 | pk | 0 | task_id |

### model_registry

```sql
CREATE TABLE model_registry (
    model_id TEXT NOT NULL PRIMARY KEY,
    provider_type TEXT NOT NULL,
    vram_base_mb INTEGER NOT NULL,
    kv_cache_cost_per_k INTEGER NOT NULL,
    max_context_window INTEGER NOT NULL,
    specialty_tags TEXT NOT NULL,
    score_lmarena INTEGER NOT NULL,
    cost_input_per_m REAL NOT NULL,
    cost_output_per_m REAL NOT NULL,
    is_active INTEGER NOT NULL,
    ema_latency_ms INTEGER NOT NULL,
    success_rate_ema REAL NOT NULL
)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | model_id | TEXT | 1 |  | 1 |
| 1 | provider_type | TEXT | 1 |  | 0 |
| 2 | vram_base_mb | INTEGER | 1 |  | 0 |
| 3 | kv_cache_cost_per_k | INTEGER | 1 |  | 0 |
| 4 | max_context_window | INTEGER | 1 |  | 0 |
| 5 | specialty_tags | TEXT | 1 |  | 0 |
| 6 | score_lmarena | INTEGER | 1 |  | 0 |
| 7 | cost_input_per_m | REAL | 1 |  | 0 |
| 8 | cost_output_per_m | REAL | 1 |  | 0 |
| 9 | is_active | INTEGER | 1 |  | 0 |
| 10 | ema_latency_ms | INTEGER | 1 |  | 0 |
| 11 | success_rate_ema | REAL | 1 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_model_registry_1 | 1 | pk | 0 | model_id |

### pacotes_destilados

```sql
CREATE TABLE pacotes_destilados (
            package_id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo_id TEXT NOT NULL,
            package_name TEXT NOT NULL,
            payload_package TEXT NOT NULL,
            timestamp_empacotamento INTEGER NOT NULL,
            UNIQUE(repo_id, package_name)
        )
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | package_id | INTEGER | 0 |  | 1 |
| 1 | repo_id | TEXT | 1 |  | 0 |
| 2 | package_name | TEXT | 1 |  | 0 |
| 3 | payload_package | TEXT | 1 |  | 0 |
| 4 | timestamp_empacotamento | INTEGER | 1 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_pacotes_destilados_1 | 1 | u | 0 | repo_id, package_name |

### repo_heuristics

```sql
CREATE TABLE repo_heuristics (
    project_name TEXT NOT NULL PRIMARY KEY REFERENCES repositorios(project_name),
    repo_url TEXT NOT NULL UNIQUE,
    repo_version TEXT NOT NULL,
    ultima_versao_online TEXT,
    lote_id TEXT NOT NULL,
    data_ultima_analise INTEGER NOT NULL,
    analise_origem TEXT NOT NULL,
    declared_description TEXT NOT NULL,
    proposta_original_resumo TEXT NOT NULL,
    stack_base TEXT NOT NULL,
    licenca TEXT,
    lente_a_sentido_prod_ux TEXT,
    lente_b_estrutura_arq TEXT,
    lente_c_realidade_ops TEXT,
    visao_do_enxame TEXT NOT NULL,
    justificativa_decisao TEXT NOT NULL,
    executive_verdict TEXT NOT NULL,
    classificacao_terminal TEXT NOT NULL,
    acao_de_canibalizacao TEXT NOT NULL,
    categoria_arquitetural TEXT NOT NULL,
    horizonte_extracao TEXT NOT NULL,
    tipo_integracao TEXT NOT NULL,
    categoria_nuance_tecnica TEXT NOT NULL,
    integracao_papel_exato TEXT NOT NULL,
    ouro_a_extrair TEXT NOT NULL,
    deep_pattern TEXT NOT NULL,
    transplantable_core TEXT NOT NULL,
    logic_math_heuristic TEXT NOT NULL,
    real_structural_problem TEXT NOT NULL,
    must_components_prod_ux TEXT NOT NULL,
    must_components_arq TEXT NOT NULL,
    must_components_ops TEXT NOT NULL,
    detected_toxic_deps TEXT NOT NULL,
    do_not_absorb TEXT NOT NULL,
    where_ai_should_not_enter TEXT NOT NULL,
    bare_metal_fit TEXT NOT NULL,
    extractability_level TEXT NOT NULL,
    operability_level TEXT NOT NULL,
    entropy_risk TEXT NOT NULL,
    design_misuse_risk TEXT NOT NULL,
    intrinsic_ethics_risk TEXT NOT NULL,
    discipline_dependency TEXT NOT NULL,
    risco_principal TEXT NOT NULL,
    risco_linha_vermelha TEXT NOT NULL,
    observacoes TEXT NOT NULL,
    score_final REAL NOT NULL,
    score_fit_geral_soda REAL NOT NULL,
    score_philosophical_fit INTEGER NOT NULL,
    score_bare_metal_fit INTEGER NOT NULL,
    score_architectural_extractability INTEGER NOT NULL,
    score_operability INTEGER NOT NULL,
    score_creep_risk INTEGER NOT NULL,
    score_runtime_sovereignty INTEGER NOT NULL,
    score_model_logic_value INTEGER NOT NULL,
    score_ethics_safety INTEGER NOT NULL,
    score_intrinsic_risk INTEGER NOT NULL,
    capability_nature_primary TEXT NOT NULL,
    architectural_topology TEXT NOT NULL,
    runtime_sovereignty_fit TEXT NOT NULL,
    local_first_fit TEXT NOT NULL,
    temporal_stability TEXT NOT NULL,
    adoptability_level TEXT NOT NULL,
    longitudinal_sustainability TEXT NOT NULL,
    abandonment_risk TEXT NOT NULL,
    maintenance_burden TEXT NOT NULL,
    onboarding_friction TEXT NOT NULL,
    observability_operational TEXT NOT NULL,
    recoverability_level TEXT NOT NULL,
    degradation_behavior TEXT NOT NULL,
    curation_burden TEXT NOT NULL,
    time_to_first_clear_value TEXT NOT NULL,
    imperfection_tolerance TEXT NOT NULL,
    evolution_cost TEXT NOT NULL,
    regulatory_risk TEXT NOT NULL,
    score_architectural_priority REAL NOT NULL,
    score_human_product_priority REAL NOT NULL,
    score_absorption_readiness REAL NOT NULL,
    score_operational_priority REAL NOT NULL,
    score_sustainability_adjusted_fit REAL NOT NULL,
    valid_from INTEGER NOT NULL,
    valid_to INTEGER,
    embargo_status INTEGER NOT NULL
, status_atualizacao TEXT NOT NULL DEFAULT 'CONCLUIDO', status_fase TEXT NOT NULL DEFAULT 'F4')
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | project_name | TEXT | 1 |  | 1 |
| 1 | repo_url | TEXT | 1 |  | 0 |
| 2 | repo_version | TEXT | 1 |  | 0 |
| 3 | ultima_versao_online | TEXT | 0 |  | 0 |
| 4 | lote_id | TEXT | 1 |  | 0 |
| 5 | data_ultima_analise | INTEGER | 1 |  | 0 |
| 6 | analise_origem | TEXT | 1 |  | 0 |
| 7 | declared_description | TEXT | 1 |  | 0 |
| 8 | proposta_original_resumo | TEXT | 1 |  | 0 |
| 9 | stack_base | TEXT | 1 |  | 0 |
| 10 | licenca | TEXT | 0 |  | 0 |
| 11 | lente_a_sentido_prod_ux | TEXT | 0 |  | 0 |
| 12 | lente_b_estrutura_arq | TEXT | 0 |  | 0 |
| 13 | lente_c_realidade_ops | TEXT | 0 |  | 0 |
| 14 | visao_do_enxame | TEXT | 1 |  | 0 |
| 15 | justificativa_decisao | TEXT | 1 |  | 0 |
| 16 | executive_verdict | TEXT | 1 |  | 0 |
| 17 | classificacao_terminal | TEXT | 1 |  | 0 |
| 18 | acao_de_canibalizacao | TEXT | 1 |  | 0 |
| 19 | categoria_arquitetural | TEXT | 1 |  | 0 |
| 20 | horizonte_extracao | TEXT | 1 |  | 0 |
| 21 | tipo_integracao | TEXT | 1 |  | 0 |
| 22 | categoria_nuance_tecnica | TEXT | 1 |  | 0 |
| 23 | integracao_papel_exato | TEXT | 1 |  | 0 |
| 24 | ouro_a_extrair | TEXT | 1 |  | 0 |
| 25 | deep_pattern | TEXT | 1 |  | 0 |
| 26 | transplantable_core | TEXT | 1 |  | 0 |
| 27 | logic_math_heuristic | TEXT | 1 |  | 0 |
| 28 | real_structural_problem | TEXT | 1 |  | 0 |
| 29 | must_components_prod_ux | TEXT | 1 |  | 0 |
| 30 | must_components_arq | TEXT | 1 |  | 0 |
| 31 | must_components_ops | TEXT | 1 |  | 0 |
| 32 | detected_toxic_deps | TEXT | 1 |  | 0 |
| 33 | do_not_absorb | TEXT | 1 |  | 0 |
| 34 | where_ai_should_not_enter | TEXT | 1 |  | 0 |
| 35 | bare_metal_fit | TEXT | 1 |  | 0 |
| 36 | extractability_level | TEXT | 1 |  | 0 |
| 37 | operability_level | TEXT | 1 |  | 0 |
| 38 | entropy_risk | TEXT | 1 |  | 0 |
| 39 | design_misuse_risk | TEXT | 1 |  | 0 |
| 40 | intrinsic_ethics_risk | TEXT | 1 |  | 0 |
| 41 | discipline_dependency | TEXT | 1 |  | 0 |
| 42 | risco_principal | TEXT | 1 |  | 0 |
| 43 | risco_linha_vermelha | TEXT | 1 |  | 0 |
| 44 | observacoes | TEXT | 1 |  | 0 |
| 45 | score_final | REAL | 1 |  | 0 |
| 46 | score_fit_geral_soda | REAL | 1 |  | 0 |
| 47 | score_philosophical_fit | INTEGER | 1 |  | 0 |
| 48 | score_bare_metal_fit | INTEGER | 1 |  | 0 |
| 49 | score_architectural_extractability | INTEGER | 1 |  | 0 |
| 50 | score_operability | INTEGER | 1 |  | 0 |
| 51 | score_creep_risk | INTEGER | 1 |  | 0 |
| 52 | score_runtime_sovereignty | INTEGER | 1 |  | 0 |
| 53 | score_model_logic_value | INTEGER | 1 |  | 0 |
| 54 | score_ethics_safety | INTEGER | 1 |  | 0 |
| 55 | score_intrinsic_risk | INTEGER | 1 |  | 0 |
| 56 | capability_nature_primary | TEXT | 1 |  | 0 |
| 57 | architectural_topology | TEXT | 1 |  | 0 |
| 58 | runtime_sovereignty_fit | TEXT | 1 |  | 0 |
| 59 | local_first_fit | TEXT | 1 |  | 0 |
| 60 | temporal_stability | TEXT | 1 |  | 0 |
| 61 | adoptability_level | TEXT | 1 |  | 0 |
| 62 | longitudinal_sustainability | TEXT | 1 |  | 0 |
| 63 | abandonment_risk | TEXT | 1 |  | 0 |
| 64 | maintenance_burden | TEXT | 1 |  | 0 |
| 65 | onboarding_friction | TEXT | 1 |  | 0 |
| 66 | observability_operational | TEXT | 1 |  | 0 |
| 67 | recoverability_level | TEXT | 1 |  | 0 |
| 68 | degradation_behavior | TEXT | 1 |  | 0 |
| 69 | curation_burden | TEXT | 1 |  | 0 |
| 70 | time_to_first_clear_value | TEXT | 1 |  | 0 |
| 71 | imperfection_tolerance | TEXT | 1 |  | 0 |
| 72 | evolution_cost | TEXT | 1 |  | 0 |
| 73 | regulatory_risk | TEXT | 1 |  | 0 |
| 74 | score_architectural_priority | REAL | 1 |  | 0 |
| 75 | score_human_product_priority | REAL | 1 |  | 0 |
| 76 | score_absorption_readiness | REAL | 1 |  | 0 |
| 77 | score_operational_priority | REAL | 1 |  | 0 |
| 78 | score_sustainability_adjusted_fit | REAL | 1 |  | 0 |
| 79 | valid_from | INTEGER | 1 |  | 0 |
| 80 | valid_to | INTEGER | 0 |  | 0 |
| 81 | embargo_status | INTEGER | 1 |  | 0 |
| 82 | status_atualizacao | TEXT | 1 | 'CONCLUIDO' | 0 |
| 83 | status_fase | TEXT | 1 | 'F4' | 0 |

**Foreign Keys**
| id | seq | table | from | to | on_update | on_delete | match |
|---:|---:|---|---|---|---|---|---|
| 0 | 0 | repositorios | project_name | project_name | NO ACTION | NO ACTION | NONE |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_repo_heuristics_2 | 1 | u | 0 | repo_url |
| sqlite_autoindex_repo_heuristics_1 | 1 | pk | 0 | project_name |

### repo_heuristics_justifications

```sql
CREATE TABLE repo_heuristics_justifications (
                project_name TEXT NOT NULL,
                block INTEGER NOT NULL,
                justifications_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (project_name, block)
            )
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | project_name | TEXT | 1 |  | 1 |
| 1 | block | INTEGER | 1 |  | 2 |
| 2 | justifications_json | TEXT | 1 |  | 0 |
| 3 | created_at | INTEGER | 1 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_repo_heuristics_justifications_1 | 1 | pk | 0 | project_name, block |

### repositorios

```sql
CREATE TABLE repositorios (
    project_name TEXT NOT NULL PRIMARY KEY,
    lote_id TEXT NOT NULL,
    repo_url TEXT NOT NULL UNIQUE,
    soda_universal_uuid TEXT NOT NULL UNIQUE,
    status_processamento TEXT NOT NULL,
    timestamp_fase_1 INTEGER,
    timestamp_fase_3 INTEGER,
    retry_count INTEGER NOT NULL
, repo_version TEXT, ultima_versao_online TEXT)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | project_name | TEXT | 1 |  | 1 |
| 1 | lote_id | TEXT | 1 |  | 0 |
| 2 | repo_url | TEXT | 1 |  | 0 |
| 3 | soda_universal_uuid | TEXT | 1 |  | 0 |
| 4 | status_processamento | TEXT | 1 |  | 0 |
| 5 | timestamp_fase_1 | INTEGER | 0 |  | 0 |
| 6 | timestamp_fase_3 | INTEGER | 0 |  | 0 |
| 7 | retry_count | INTEGER | 1 |  | 0 |
| 8 | repo_version | TEXT | 0 |  | 0 |
| 9 | ultima_versao_online | TEXT | 0 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_repositorios_3 | 1 | u | 0 | soda_universal_uuid |
| sqlite_autoindex_repositorios_2 | 1 | u | 0 | repo_url |
| sqlite_autoindex_repositorios_1 | 1 | pk | 0 | project_name |

### weevolve_learnings

```sql
CREATE TABLE weevolve_learnings (
    learning_id TEXT NOT NULL PRIMARY KEY,
    the_insight TEXT NOT NULL,
    why_this_matters TEXT NOT NULL,
    recognition_pattern TEXT NOT NULL,
    the_approach TEXT NOT NULL,
    timestamp_aprendizado INTEGER NOT NULL
)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | learning_id | TEXT | 1 |  | 1 |
| 1 | the_insight | TEXT | 1 |  | 0 |
| 2 | why_this_matters | TEXT | 1 |  | 0 |
| 3 | recognition_pattern | TEXT | 1 |  | 0 |
| 4 | the_approach | TEXT | 1 |  | 0 |
| 5 | timestamp_aprendizado | INTEGER | 1 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_weevolve_learnings_1 | 1 | pk | 0 | learning_id |

## VIEWS

### action_matrix

```sql
CREATE VIEW action_matrix AS
    SELECT project_name, acao_de_canibalizacao, transplantable_core, score_architectural_priority, score_absorption_readiness
    FROM repo_heuristics
    WHERE classificacao_terminal IN ('STACK_CORE_PLANO_A', 'INTEGRATE_AS_COMPONENT')
```

### quarantine_radar

```sql
CREATE VIEW quarantine_radar AS
    SELECT project_name, design_misuse_risk, entropy_risk, intrinsic_ethics_risk, risco_principal
    FROM repo_heuristics
    WHERE design_misuse_risk IN ('HIGH', 'CRITICAL')
       OR entropy_risk IN ('HIGH', 'CRITICAL')
       OR intrinsic_ethics_risk IN ('HIGH', 'CRITICAL')
```

### soda_graph_topology

```sql
CREATE VIEW soda_graph_topology AS
    SELECT project_name, stack_base, architectural_topology, capability_nature_primary
    FROM repo_heuristics
```

## INDEXS

### idx_artefatos_repo_tipo

```sql
CREATE UNIQUE INDEX idx_artefatos_repo_tipo
         ON artefatos_brutos(repo_id, artifact_type)
```

---

# DB_STATE_REPORT — soda_state.db

- Generated at (UTC): 2026-05-25T23:59:54Z
- Database path: c:\Users\rosas\Dev_Projects\genesis_mc\.soda_data\soda_state.db
- SQLite version: 3.49.1
- PRAGMA user_version: 0
- PRAGMA foreign_keys: 0

## Inventory

- Objects: table=7, index=6, trigger=3

## TABLES

### entities

```sql
CREATE TABLE entities (
    name TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    observations TEXT NOT NULL
) STRICT
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | name | TEXT | 1 |  | 1 |
| 1 | entity_type | TEXT | 1 |  | 0 |
| 2 | observations | TEXT | 1 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| idx_entity_type | 0 | c | 0 | entity_type |
| sqlite_autoindex_entities_1 | 1 | pk | 0 | name |

### entities_fts

```sql
CREATE VIRTUAL TABLE entities_fts USING fts5(
    name,
    entity_type,
    observations,
    content='entities',
    content_rowid='rowid'
)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | name |  | 0 |  | 0 |
| 1 | entity_type |  | 0 |  | 0 |
| 2 | observations |  | 0 |  | 0 |

### entities_fts_config

```sql
CREATE TABLE 'entities_fts_config'(k PRIMARY KEY, v) WITHOUT ROWID
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | k |  | 1 |  | 1 |
| 1 | v |  | 0 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_entities_fts_config_1 | 1 | pk | 0 | k |

### entities_fts_data

```sql
CREATE TABLE 'entities_fts_data'(id INTEGER PRIMARY KEY, block BLOB)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | id | INTEGER | 0 |  | 1 |
| 1 | block | BLOB | 0 |  | 0 |

### entities_fts_docsize

```sql
CREATE TABLE 'entities_fts_docsize'(id INTEGER PRIMARY KEY, sz BLOB)
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | id | INTEGER | 0 |  | 1 |
| 1 | sz | BLOB | 0 |  | 0 |

### entities_fts_idx

```sql
CREATE TABLE 'entities_fts_idx'(segid, term, pgno, PRIMARY KEY(segid, term)) WITHOUT ROWID
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | segid |  | 1 |  | 1 |
| 1 | term |  | 1 |  | 2 |
| 2 | pgno |  | 0 |  | 0 |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| sqlite_autoindex_entities_fts_idx_1 | 1 | pk | 0 | segid, term |

### relations

```sql
CREATE TABLE relations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_entity TEXT NOT NULL,
    to_entity TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    UNIQUE(from_entity, to_entity, relation_type),
    FOREIGN KEY(from_entity) REFERENCES entities(name) ON DELETE CASCADE,
    FOREIGN KEY(to_entity) REFERENCES entities(name) ON DELETE CASCADE
) STRICT
```

**Columns**
| cid | name | type | notnull | dflt_value | pk |
|---:|---|---|---:|---|---:|
| 0 | id | INTEGER | 0 |  | 1 |
| 1 | from_entity | TEXT | 1 |  | 0 |
| 2 | to_entity | TEXT | 1 |  | 0 |
| 3 | relation_type | TEXT | 1 |  | 0 |

**Foreign Keys**
| id | seq | table | from | to | on_update | on_delete | match |
|---:|---:|---|---|---|---|---|---|
| 0 | 0 | entities | to_entity | name | NO ACTION | CASCADE | NONE |
| 1 | 0 | entities | from_entity | name | NO ACTION | CASCADE | NONE |

**Indexes**
| name | unique | origin | partial | columns |
|---|---:|---|---:|---|
| idx_relations_to_type | 0 | c | 0 | to_entity, relation_type |
| idx_relations_from_type | 0 | c | 0 | from_entity, relation_type |
| idx_relation_type | 0 | c | 0 | relation_type |
| idx_to | 0 | c | 0 | to_entity |
| idx_from | 0 | c | 0 | from_entity |
| sqlite_autoindex_relations_1 | 1 | u | 0 | from_entity, to_entity, relation_type |

## INDEXS

### idx_entity_type

```sql
CREATE INDEX idx_entity_type ON entities(entity_type)
```

### idx_from

```sql
CREATE INDEX idx_from ON relations(from_entity)
```

### idx_relation_type

```sql
CREATE INDEX idx_relation_type ON relations(relation_type)
```

### idx_relations_from_type

```sql
CREATE INDEX idx_relations_from_type ON relations(from_entity, relation_type)
```

### idx_relations_to_type

```sql
CREATE INDEX idx_relations_to_type ON relations(to_entity, relation_type)
```

### idx_to

```sql
CREATE INDEX idx_to ON relations(to_entity)
```

## TRIGGERS

### entities_ad

```sql
CREATE TRIGGER entities_ad AFTER DELETE ON entities BEGIN
    INSERT INTO entities_fts(entities_fts, rowid, name, entity_type, observations)
    VALUES ('delete', old.rowid, old.name, old.entity_type, old.observations);
END
```

### entities_ai

```sql
CREATE TRIGGER entities_ai AFTER INSERT ON entities BEGIN
    INSERT INTO entities_fts(rowid, name, entity_type, observations)
    VALUES (new.rowid, new.name, new.entity_type, new.observations);
END
```

### entities_au

```sql
CREATE TRIGGER entities_au AFTER UPDATE ON entities BEGIN
    INSERT INTO entities_fts(entities_fts, rowid, name, entity_type, observations)
    VALUES ('delete', old.rowid, old.name, old.entity_type, old.observations);
    INSERT INTO entities_fts(rowid, name, entity_type, observations)
    VALUES (new.rowid, new.name, new.entity_type, new.observations);
END
```

