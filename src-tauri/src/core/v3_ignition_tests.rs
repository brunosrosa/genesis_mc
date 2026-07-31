#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use crate::core::file_locker::{acquire_file_lock, atomic_write_file};
    #[cfg(feature = "llama_backend")]
    use crate::core::llama_engine::disable_model_in_sqlite;

    #[tokio::test]
    async fn test_atomic_souls_edit_concurrency() {
        let temp_dir = tempfile::tempdir().expect("Falha ao criar tempdir");
        let file_path = temp_dir.path().join("concurrent_edit_target.rs");
        
        let initial_content = "// BASE FILE HEADER\nfn main() {\n    // souls-stub: edit_me\n}\n";
        tokio::fs::write(&file_path, initial_content).await.expect("Falha ao escrever arquivo inicial");

        let path_arc = Arc::new(file_path.clone());
        let mut handles = vec![];

        for i in 0..5 {
            let path_clone = Arc::clone(&path_arc);
            let handle = tokio::spawn(async move {
                let lock = acquire_file_lock(&path_clone);
                let _guard = lock.lock().await;

                let content = tokio::fs::read_to_string(&*path_clone).await.expect("Leitura falhou");
                let new_content = format!("{}\n// edit entry {}\n", content, i);
                atomic_write_file(&*path_clone, &new_content).await.expect("Escrita atômica falhou");
            });
            handles.push(handle);
        }

        for h in handles {
            h.await.expect("Task falhou");
        }

        let final_content = tokio::fs::read_to_string(&file_path).await.expect("Leitura final falhou");
        for i in 0..5 {
            assert!(
                final_content.contains(&format!("// edit entry {}", i)),
                "A alteração da thread {} deve estar presente no arquivo final sem perdas",
                i
            );
        }
    }

    #[test]
    fn test_souls_fill_vram_awareness() {
        let sample_code = r#"
            // Comentário extenso que deve ser podido pelo CodeCompressor quando o limite de VRAM estiver apertado
            pub fn verbose_function_with_lots_of_comments() -> i32 {
                // Outro comentário interno
                let a = 10;
                let b = 20;
                a + b
            }
        "#;
        let compressed = crate::core::headroom_engine::CodeCompressor::compress_ast_zero_copy(sample_code);
        assert!(
            compressed.len() < sample_code.len(),
            "O compressor AST deve reduzir o tamanho do código quando acionado na Zona Vermelha (>80%)"
        );
    }

    #[cfg(feature = "llama_backend")]
    #[tokio::test]
    async fn test_safe_fallback_guardrail() {
        let test_db = crate::core::model_registry::resolve_db_path();
        if let Ok(conn) = rusqlite::Connection::open(&test_db) {
            let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
            let _ = conn.execute_batch("PRAGMA journal_mode=WAL;");
            let _ = conn.execute(
                "CREATE TABLE IF NOT EXISTS model_registry (
                    model_id TEXT PRIMARY KEY,
                    file_path TEXT,
                    max_context_length INTEGER,
                    is_active INTEGER
                )",
                [],
            );
            let _ = conn.execute(
                "INSERT OR REPLACE INTO model_registry (model_id, file_path, max_context_length, is_active)
                 VALUES ('test_crash_model', 'test_crash_model.gguf', 4096, 1)",
                [],
            );
        }

        disable_model_in_sqlite("test_crash_model.gguf");

        if let Ok(conn) = rusqlite::Connection::open(&test_db) {
            let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
            let active: i32 = conn.query_row(
                "SELECT is_active FROM model_registry WHERE file_path = 'test_crash_model.gguf'",
                [],
                |row| row.get(0),
            ).unwrap_or(0);
            assert_eq!(active, 0, "O modelo defeituoso deve ser desativado (is_active = 0) no SQLite");
        }
    }
}
