//! Routes: Mapeamento de contrato IPC Tauri <-> Svelte via Regex.
//!
//! Dois lados do mesmo contrato:
//! - **Backend (Rust/Tauri):** comandos anotados `#[tauri::command]`.
//! - **Frontend (Svelte/TS):** invocacoes `invoke("comando", ...)`.

use crate::cognition::memory_graph::errors::CognitiveError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use walkdir::WalkDir;

/// Regex compilada uma unica vez (OnceLock) para detectar comandos Tauri.
fn tauri_command_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?xm)
            ^\s*\#\s*\[\s*tauri::command\s*(?:\([^)]*\))?\s*\][^\n]*\n
            \s*(?:pub\s+)?(?:async\s+)?fn\s+([a-z][a-z0-9_]*)\s*\(",
        )
        .expect("regex de tauri::command valida")
    })
}

/// Regex compilada uma unica vez para detectar invokes Svelte/TS.
///
/// Aceita aspas simples OU duplas. Sem backreferences (Rust `regex`
/// nao suporta), entao a alternacao cobre ambos os casos.
fn svelte_invoke_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?:'(?P<cmd_s>[a-z][a-z0-9_]*)'|"(?P<cmd_d>[a-z][a-z0-9_]*)")"#)
            .expect("regex de invoke valida")
    })
}

/// Entrada individual do relatorio de rotas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteEntry {
    /// Nome canonico do comando (Rust) ou invoke (Svelte/TS).
    pub name: String,
    /// Caminho do arquivo onde foi detectado.
    pub file: String,
    /// Linha aproximada (1-indexed) — `0` se nao aplicavel.
    pub line: usize,
}

/// Relatorio consolidado de rotas (backend + frontend + orphans).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RouteReport {
    /// Comandos Tauri detectados no backend Rust.
    pub backend: Vec<RouteEntry>,
    /// Invokes detectados no frontend Svelte/TS.
    pub frontend: Vec<RouteEntry>,
    /// Comandos backend sem invoke frontend equivalente (potenciais
    /// candidatos a remocao ou documentacao).
    pub orphans: Vec<String>,
    /// Invokes frontend sem comando backend equivalente (potenciais
    /// chamadas mortas — alvo de auditoria).
    pub dead_calls: Vec<String>,
}

/// Varre o monorepo e produz o relatorio de rotas.
///
/// `root` e o diretorio raiz a ser vasculhado. Internamente:
/// 1. Acha todos os `.rs` e procura por `#[tauri::command]` + `fn nome`.
/// 2. Acha todos os `.svelte` e `.ts` e procura por `invoke("nome", ...)`.
/// 3. Calcula `orphans` (backend - frontend) e `dead_calls` (frontend - backend).
pub fn scan_routes(root: &Path) -> Result<RouteReport, CognitiveError> {
    let cmd_re = tauri_command_regex();
    let inv_re = svelte_invoke_regex();

    let mut backend: Vec<RouteEntry> = Vec::new();
    let mut frontend: Vec<RouteEntry> = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !(name == "target"
                || name == "vendor"
                || name == "third_party"
                || name == ".git"
                || name == ".souls_cache"
                || name == ".souls_sandbox"
                || name == ".souls_data"
                || name == "node_modules"
                || name == "dist")
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        match ext {
            "rs" => {
                for cap in cmd_re.captures_iter(&content) {
                    if let Some(name) = cap.get(1) {
                        let line = content[..name.start()].lines().count();
                        backend.push(RouteEntry {
                            name: name.as_str().to_string(),
                            file: rel.clone(),
                            line,
                        });
                    }
                }
            }
            "svelte" | "ts" | "tsx" | "js" | "jsx" => {
                for cap in inv_re.captures_iter(&content) {
                    let name_opt = cap.name("cmd_s").or_else(|| cap.name("cmd_d"));
                    if let Some(name) = name_opt {
                        let line = content[..name.start()].lines().count();
                        // Heuristica leve: exige `invoke(` ou `await invoke(` antes
                        // da aspa. Filtra falso-positivos como strings soltas.
                        let prefix_start = name.start().saturating_sub(32);
                        let prefix = &content[prefix_start..name.start()];
                        if prefix.contains("invoke(") {
                            frontend.push(RouteEntry {
                                name: name.as_str().to_string(),
                                file: rel.clone(),
                                line,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Calcula orphans e dead_calls.
    let backend_names: std::collections::BTreeSet<String> =
        backend.iter().map(|e| e.name.clone()).collect();
    let frontend_names: std::collections::BTreeSet<String> =
        frontend.iter().map(|e| e.name.clone()).collect();
    let orphans: Vec<String> = backend_names
        .difference(&frontend_names)
        .cloned()
        .collect();
    let dead_calls: Vec<String> = frontend_names
        .difference(&backend_names)
        .cloned()
        .collect();

    Ok(RouteReport {
        backend,
        frontend,
        orphans,
        dead_calls,
    })
}
