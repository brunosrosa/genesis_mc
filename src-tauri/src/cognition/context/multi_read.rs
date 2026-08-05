// SOULS-CANIBALIZED Marco 3.6: Tool `souls_multi_read` — leitura concorrente
// em lote de múltiplos arquivos com compressão CCR por arquivo.
//
// Lê N arquivos em paralelo via `tokio::fs` + `futures::future::join_all`,
// aplica `compress_with_dedup` em cada conteúdo, e retorna um mapeamento
// `Filepath -> CompactedContent`.
//
// Zero I/O block: cada leitura dispara um `tokio::spawn` independente.
// Zero alocação extra: usa Vec<String> por arquivo, não buffers global.

use std::path::Path;

use super::dedup::compress_with_dedup;

/// Resultado de compactação de UM arquivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCompaction {
    /// Caminho original (preservado como fornecido pelo caller).
    pub filepath: String,
    /// Conteúdo compactado via `compress_with_dedup`.
    pub compacted: String,
    /// Bytes lidos do disco (raw, antes da compressão).
    pub original_bytes: usize,
    /// Bytes do conteúdo compactado (após compressão).
    pub compacted_bytes: usize,
    /// Erro de leitura, se houver. Quando presente, `compacted` é vazia
    /// e o caller deve reportar a falha.
    pub error: Option<String>,
}

/// Lê múltiplos arquivos de forma concorrente, aplicando compressão CCR em
/// cada um. A ordem do retorno é a mesma ordem do input.
///
/// Arquivos com erro de leitura (não existe, permissão negada, binário)
/// retornam `FileCompaction { error: Some(...), compacted: "" }` mas NÃO
/// abortam o batch (fail-soft).
pub async fn multi_read_concurrent<I, P>(paths: I) -> Vec<FileCompaction>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    // Materializa os paths upfront para evitar problemas de lifetime no spawn.
    let path_strs: Vec<String> = paths
        .into_iter()
        .map(|p| p.as_ref().to_string_lossy().to_string())
        .collect();

    let mut handles = Vec::with_capacity(path_strs.len());
    for p in path_strs {
        let handle = tokio::spawn(async move {
            let path = Path::new(&p);
            let raw = match tokio::fs::read_to_string(path).await {
                Ok(s) => s,
                Err(e) => {
                    return FileCompaction {
                        filepath: p,
                        compacted: String::new(),
                        original_bytes: 0,
                        compacted_bytes: 0,
                        error: Some(format!("read_error: {e}")),
                    };
                }
            };
            let original_bytes = raw.len();
            let (compacted, _stats) = compress_with_dedup(&raw);
            let compacted_bytes = compacted.len();
            FileCompaction {
                filepath: p,
                compacted,
                original_bytes,
                compacted_bytes,
                error: None,
            }
        });
        handles.push(handle);
    }

    let mut out = Vec::with_capacity(handles.len());
    for h in handles {
        match h.await {
            Ok(fc) => out.push(fc),
            Err(e) => {
                // JoinError improvável (panic no task). Reporta como erro isolado.
                out.push(FileCompaction {
                    filepath: String::new(),
                    compacted: String::new(),
                    original_bytes: 0,
                    compacted_bytes: 0,
                    error: Some(format!("join_error: {e}")),
                });
            }
        }
    }
    out
}
