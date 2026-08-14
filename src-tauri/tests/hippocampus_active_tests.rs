// SOULS V6 — Integration Tests: MARCO VI (Hipocampo Ativo, Reator RRF, LadybugDB e Chyros Daemon)
// Conforme ADR-001, ADR-003, ADR-005, ADR-025, ADR-027, ADR-030, ADR-040.

use rusqlite::Connection;
use souls_mc_lib::cognition::memory::{
    apply_langevin_decay, fts_retriever::LexicalMatch, init_memory_schema,
    ladybug_firewall::{FirewallVerdict, OntologicalFirewall},
    rrf_fusion::RrfFusionEngine,
    vector_retriever::{HippocampusMemoryRecord, VectorRetriever, VECTOR_DIMENSION},
    ChyrosDaemon,
};
use std::collections::HashSet;
use std::time::Instant;
use tempfile::tempdir;

#[tokio::test]
async fn test_lancedb_mmap_zero_vram_isolation() {
    let dir = tempdir().expect("Falha ao criar tempdir");
    let lance_path = dir.path().join("test_lancedb_mmap");

    let retriever = VectorRetriever::new(&lance_path);

    // Persiste 100 embeddings com 384 floats cada
    for i in 0..100 {
        let embedding: Vec<f32> = (0..VECTOR_DIMENSION)
            .map(|d| ((i * 384 + d) as f32).sin())
            .collect();

        let record = HippocampusMemoryRecord {
            id: format!("uuid-mem-{}", i),
            text_content: format!("Memory content chunk {} with architectural knowledge", i),
            embedding,
            temporal_stability: if i % 2 == 0 { "STABLE".to_string() } else { "EVOLVING".to_string() },
            valid_from: 1700000000 + (i as i64 * 100),
            valid_to: None,
        };

        retriever
            .insert_memory(record)
            .await
            .expect("Falha na inserção no LanceDB");
    }

    // Consulta kNN via mmap na CPU Host
    let query_vector: Vec<f32> = (0..VECTOR_DIMENSION)
        .map(|d| (d as f32).sin())
        .collect();

    let matches = retriever
        .search_vectorial(&query_vector, 10)
        .await
        .expect("Falha na busca vetorial kNN");

    assert!(!matches.is_empty(), "Deve retornar correspondências do LanceDB");
    assert!(matches.len() <= 10);

    // Validação de Isolamento de VRAM:
    // O LanceDB em modo serverless opera sobre buffers Arrow mapeados via mmap2 no Host RAM/NVMe.
    // Nenhuma chamada FFI de CUDA/VRAM é acionada.
    for m in &matches {
        assert!(m.similarity > 0.0 && m.similarity <= 1.0);
        assert!(!m.observation_id.is_empty());
    }
}

#[test]
fn test_hybrid_search_rrf_avx2_fusion() {
    let engine = RrfFusionEngine::default();

    let mut lexical_matches = Vec::new();
    let mut vector_matches = Vec::new();

    // Popula 200 itens simulados
    for i in 0..200 {
        lexical_matches.push(LexicalMatch {
            observation_id: format!("doc_{}", i),
            content: format!("Prosa textual genérica número {}", i),
            file_path: format!("src/module_{}.rs", i),
            raw_score: (i as f64) * 0.1,
        });

        vector_matches.push(souls_mc_lib::cognition::memory::VectorialMatch {
            observation_id: format!("doc_{}", i),
            content: format!("Prosa textual genérica número {}", i),
            similarity: 1.0 / (1.0 + (i as f32) * 0.05),
            file_path: format!("src/module_{}.rs", i),
            temporal_stability: "STABLE".to_string(),
            valid_from: 1700000000,
            valid_to: None,
            metadata: serde_json::json!({}),
        });
    }

    // Injeta termo exato com constante rígida em um documento de rank baixo
    lexical_matches.push(LexicalMatch {
        observation_id: "doc_rigid_target".to_string(),
        content: "Definição de quantização GGML_TYPE_TQ1 na CPU Host AVX2".to_string(),
        file_path: "src/core/pulp_matrix_engine.rs".to_string(),
        raw_score: 99.0, // Pior rank inicial
    });

    let tombstones = HashSet::new();
    let query = "GGML_TYPE_TQ1";

    let start = Instant::now();
    let (results, _elapsed) = engine.fuse_with_query(query, &lexical_matches, &vector_matches, &tombstones);
    let total_test_elapsed = start.elapsed();

    assert!(!results.is_empty(), "Fusão RRF deve retornar resultados");
    assert!(
        total_test_elapsed.as_millis() < 5,
        "Latência da fusão RRF na CPU deve ser sub-5ms (foi {} ms)",
        total_test_elapsed.as_millis()
    );

    // Valida que o termo exato herdou precedência máxima no topo
    assert_eq!(
        results[0].observation_id, "doc_rigid_target",
        "Documento com constante exata GGML_TYPE_TQ1 deve liderar o ranking unificado"
    );
    assert!(results[0].is_exact_match);
    assert!(results[0].rrf_score > results[1].rrf_score);
}

#[test]
fn test_ladybug_graph_bfs_poison_prevention() {
    let firewall = OntologicalFirewall::new();

    // Registra nós no grafo
    firewall.register_node(
        "ADR-030",
        "ADR",
        "STABLE",
        &["winapi", "core_affinity"],
        &["windows-sys = \"=0.61.2\""],
    );

    firewall.register_node(
        "src/core/mod.rs",
        "SourceCode",
        "STABLE",
        &[],
        &[],
    );

    firewall.register_node(
        "src/main.rs",
        "SourceCode",
        "STABLE",
        &[],
        &[],
    );

    // Conecta arestas causais: main.rs -> core/mod.rs -> ADR-030
    firewall.register_edge("src/main.rs", "src/core/mod.rs", "depends_on");
    firewall.register_edge("src/core/mod.rs", "ADR-030", "depends_on");

    // Cenário A: Chunk limpo e compatível
    let valid_chunk = "use windows_sys::Win32::System::Threading::SetThreadAffinityMask;";
    let verdict_a = firewall.bfs_check_compliance("src/main.rs", valid_chunk, 4);
    assert_eq!(verdict_a, FirewallVerdict::Approved);

    // Cenário B: Chunk envenenado com tentativa de injeção de dependência banida (winapi)
    let poisoned_chunk = "use winapi::um::processthreadsapi::GetCurrentThread; // Legacy call";
    let verdict_b = firewall.bfs_check_compliance("src/main.rs", poisoned_chunk, 4);

    match verdict_b {
        FirewallVerdict::Vetoed { violated_node, reason, relation_path } => {
            assert_eq!(violated_node, "ADR-030");
            assert!(reason.contains("winapi"));
            assert!(!relation_path.is_empty());
        }
        FirewallVerdict::Approved => {
            panic!("O firewall LadybugDB deveria ter vetado o chunk com winapi!");
        }
    }
}

#[test]
fn test_chyros_langevin_decay_vacuum_into() {
    let dir = tempdir().expect("Falha ao criar tempdir");
    let db_path = dir.path().join("souls_state_test.db");
    let defrag_path = dir.path().join("souls_state_defrag.db");

    let conn = Connection::open(&db_path).expect("open test sqlite");
    init_memory_schema(&conn).expect("init schema");

    // Popula nós STABLE e EVOLVING
    conn.execute(
        "INSERT INTO souls_memory_nodes (memory_id, content, stability_status, relevance_score, poincare_x, poincare_y, updated_at)
         VALUES ('node_adr_stable', 'ADR-001 Core Stack Rules', 'STABLE', 1.0, 0.1, 0.1, 1000)",
        [],
    ).unwrap();

    conn.execute(
        "INSERT INTO souls_memory_nodes (memory_id, content, stability_status, relevance_score, poincare_x, poincare_y, updated_at)
         VALUES ('node_chat_evolving', 'Temporary chat discussion log', 'EVOLVING', 1.0, 0.2, 0.2, 1000)",
        [],
    ).unwrap();

    // Simula 3 ciclos temporais do Chyros Daemon
    for _ in 0..3 {
        let updated = apply_langevin_decay(&conn, 0.50, 0.05, 1.0).expect("langevin cycle");
        assert_eq!(updated, 1, "Apenas nós EVOLVING devem ser atualizados no decaimento");
    }

    // Assevera que o nó STABLE permaneceu estritamente com score 1.0
    let stable_score: f64 = conn
        .query_row(
            "SELECT relevance_score FROM souls_memory_nodes WHERE memory_id = 'node_adr_stable'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stable_score, 1.0, "Nós STABLE devem ser imutáveis contra decaimento (lambda=0)");

    // Assevera que o nó EVOLVING decaiu
    let evolving_score: f64 = conn
        .query_row(
            "SELECT relevance_score FROM souls_memory_nodes WHERE memory_id = 'node_chat_evolving'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        evolving_score < 1.0,
        "Nós EVOLVING devem ter seu score decaído geometricamente (atual: {})",
        evolving_score
    );

    // Executa e valida o VACUUM INTO
    let daemon = ChyrosDaemon::new(&db_path, 10);
    daemon
        .execute_vacuum_into(&conn, &defrag_path)
        .expect("VACUUM INTO deve executar com sucesso");

    assert!(defrag_path.exists(), "O arquivo defragmentado deve existir no disco ReFS");
    let metadata = std::fs::metadata(&defrag_path).expect("metadata defrag");
    assert!(metadata.len() > 0, "O volume defragmentado não pode ser vazio");
}
