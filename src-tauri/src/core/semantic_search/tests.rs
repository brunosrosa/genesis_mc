use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn test_lancedb_mmap_zero_vram_isolation() {
    let temp_dir = TempDir::new().expect("falha ao criar tempdir para lancedb");
    let store = LanceDbVectorStore::new(temp_dir.path());

    let vram_before = query_nvml_vram_used_bytes();

    // Insere 10 registros de teste no LanceDB
    for i in 0..10 {
        let mut emb = vec![0.0_f32; VECTOR_DIMENSION as usize];
        emb[i % (VECTOR_DIMENSION as usize)] = 1.0;
        store
            .insert_record(SemanticMemoryRecord {
                id: format!("mem_{i}"),
                text_content: format!("Conteúdo de memória de teste número {i}"),
                embedding: emb,
                temporal_stability: if i % 2 == 0 { "STABLE".to_string() } else { "EVOLVING".to_string() },
                valid_from: 1000 + (i as i64 * 100),
                valid_to: None,
            })
            .await
            .expect("falha ao inserir registro");
    }

    let mut query_emb = vec![0.0_f32; VECTOR_DIMENSION as usize];
    query_emb[0] = 1.0;

    // Executa buscas vetoriais massivas
    for _ in 0..20 {
        let matches = store
            .search_vectorial(&query_emb, 5, None, None, None, false)
            .await
            .expect("falha na busca vetorial");
        assert!(!matches.is_empty(), "deve retornar resultados da busca vetorial");
    }

    // Executa busca vetorial com pré-filtro escalar restritivo (< 1000 registros / curto prazo)
    let filtered_matches = store
        .search_vectorial(&query_emb, 5, Some(1500), None, Some("STABLE"), true)
        .await
        .expect("falha na busca com bypass_vector_index");
    assert!(!filtered_matches.is_empty());
    assert_eq!(filtered_matches[0].temporal_stability, "STABLE");

    let vram_after = query_nvml_vram_used_bytes();

    // Verificação de isolamento Zero-VRAM (ADR-027): se NVML ativo, delta deve ser 0 bytes de VRAM
    if let (Some(b4), Some(aft)) = (vram_before, vram_after) {
        let vram_delta = (aft as i64 - b4 as i64).abs();
        assert_eq!(
            vram_delta, 0,
            "Violação de isolamento Zero-VRAM: oscilação de VRAM detectada ({} bytes)",
            vram_delta
        );
    }
}

#[test]
fn test_hybrid_search_rrf_avx2_fusion() {
    let reactor = HybridRrfFusionReactor::new(DEFAULT_RRF_K);

    // Gera 250 candidatos léxicos e 250 candidatos vetoriais (total 500)
    let mut lexical = Vec::with_capacity(250);
    for i in 0..250 {
        lexical.push(LexicalMatch {
            observation_id: format!("obs_lex_{i}"),
            content: format!("Lexical match observation snippet #{} with ADR-030 and windows-sys details", i),
            file_path: format!("src/module_{i}.rs"),
            raw_score: i as f64 * 0.1,
        });
    }

    let mut vectorial = Vec::with_capacity(250);
    for i in 0..250 {
        vectorial.push(VectorialMatch {
            observation_id: if i % 2 == 0 { format!("obs_lex_{}", i / 2) } else { format!("obs_vec_{i}") },
            content: format!("Vectorial match observation snippet #{} touching GGML_TYPE_TQ1", i),
            similarity: 0.95 - (i as f32 * 0.001),
            file_path: format!("src/vector_{i}.rs"),
            temporal_stability: "STABLE".to_string(),
            valid_from: 1000,
            valid_to: None,
            metadata: json!({}),
        });
    }

    let tombstones = HashSet::new();
    let query = "ADR-030";

    let (fused, elapsed) = reactor.fuse(query, &lexical, &vectorial, &tombstones);

    assert!(!fused.is_empty());
    // Latência de fusão híbrida na CPU deve ser sub-5ms (< 5000 microssegundos)
    assert!(
        elapsed < Duration::from_millis(5),
        "Latência de fusão RRF excedeu 5ms: {:?}",
        elapsed
    );

    // O item que contém o termo exato "ADR-030" deve estar no topo com pontuação amplificada
    assert!(fused[0].is_exact_match);
    assert!(fused[0].rrf_score >= EXACT_MATCH_BONUS);

    // Testa cálculo AVX2 em lote
    let ranks: Vec<f32> = (1..=64).map(|x| x as f32).collect();
    let mut scores = vec![0.0_f32; 64];
    compute_rrf_batch_avx2(&ranks, DEFAULT_RRF_K as f32, &mut scores);
    assert!((scores[0] - (1.0 / (DEFAULT_RRF_K as f32 + 1.0))).abs() < 1e-6);
}

#[test]
fn test_ladybug_graph_bfs_poison_prevention() {
    let firewall = LadybugOntologicalFirewall::new();

    // Insere uma ADR fictícia inviolável (STABLE)
    firewall.register_node(
        "ADR-999-STABLE",
        "ADR",
        "STABLE",
        &["Nodejs", "Express", "npm install -g"],
        &["Rust Tokio Bare-Metal"],
    );

    // Cria relação causal: Componente -> viola ADR-999 se usar Nodejs
    firewall.register_edge("GatewayComponent", "ADR-999-STABLE", "governed_by");

    // Chunk seguro
    let safe_chunk = UnifiedMatch {
        observation_id: "safe_01".to_string(),
        content: "Implementação nativa em Rust Tokio bare-metal IPC Zero-Copy".to_string(),
        file_path: "src/gateway.rs".to_string(),
        rrf_score: 1.0,
        lexical_rank: Some(1),
        vector_rank: Some(1),
        is_exact_match: false,
        status: "valid".to_string(),
    };

    // Chunk malicioso / contraditório
    let malicious_chunk = UnifiedMatch {
        observation_id: "poison_01".to_string(),
        content: "Use Express/Nodejs para o gateway e adicione scripts npm no package.json".to_string(),
        file_path: "src/server.js".to_string(),
        rrf_score: 2.0,
        lexical_rank: Some(2),
        vector_rank: Some(2),
        is_exact_match: false,
        status: "valid".to_string(),
    };

    let items = vec![safe_chunk.clone(), malicious_chunk.clone()];

    // Executa sanitização ontológica com BFS
    let (approved, vetoed_reasons) = firewall.sanitize_chunks(
        "GatewayComponent",
        items,
        |m| &m.content,
    );

    assert_eq!(approved.len(), 1, "Apenas o chunk seguro deve ser aprovado");
    assert_eq!(approved[0].observation_id, "safe_01");
    assert_eq!(vetoed_reasons.len(), 1, "O chunk venenoso deve ser vetado");
    assert!(
        vetoed_reasons[0].contains("ADR-999-STABLE") && vetoed_reasons[0].contains("Nodejs"),
        "A razão do veto deve apontar a violação: {}",
        vetoed_reasons[0]
    );
}
