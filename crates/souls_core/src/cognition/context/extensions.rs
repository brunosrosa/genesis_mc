//! `extensions.rs` — Tabelas canônicas de extensões e exclusões (Marco 4.0.1).
//!
//! Transplante unificado da "Alma Matemática" do
//! [`ctx_heatmap.rs`](../../third_party/lean-ctx/src/tools/ctx_heatmap.rs) e
//! do [`search.rs`](./search.rs) do próprio SOULS. O objetivo é prover
//! **um único ponto de verdade (SSOT)** para:
//!
//! 1. **Extensões de código-fonte produtivas** que o Harvester e o
//!    `tree` MCP devem considerar ao varrer um projeto.
//! 2. **Diretórios tóxicos** que devem ser **excluídos** de qualquer
//!    varredura (caches, builds, deps, VCS, ambientes virtuais).
//!
//! **Princípio DRY:** qualquer crate que precise varrer o workspace
//! deve importar daqui, nunca redefinir sua própria lista. Isso evita
//! drift semântico (e.g., esquecer de excluir `node_modules` em uma
//! nova CLI).
//!
//! **Canibalização:** as 17 extensões do `ctx_heatmap.rs` original
//! foram preservadas verbatim, complementadas com `.svelte` e `.vue`
//! (já presentes no original) e com `.yaml`/`.yml`/`.toml`/`.json`
//! para cobrir config-as-code. As exclusões do `ctx_heatmap.rs`
//! (dot, node_modules, target, dist, __pycache__, .git) foram
//! estendidas com o vocabulário do `search.rs` do SOULS
//! (`.souls_cache`, `.souls_data`, `.cargo`, `.vscode`, `.idea`)
//! e com `vendor`, `build`, `.venv`, `__snapshots__` (Rust/instr).
//!
//! **SSOT vivo:** se uma nova exclusão for necessária (e.g., `dist-newstyle`
//! do Cabal), adicione aqui e em **nenhum outro lugar**.

/// Extensões de arquivos que o Harvester e o `tree` MCP tratam como
/// "código-fonte produtivo" (i.e., devem ser indexados, vasculhados,
/// parseados).
///
/// Lista derivada verbatim do `ctx_heatmap.rs` do `lean-ctx` (17
/// extensões) + 4 entradas para config-as-code (yaml/yml/toml/json).
/// Total: **21 extensões**.
pub const SOURCE_EXTENSIONS: &[&str] = &[
    // === Código-fonte (canibalizado de ctx_heatmap.rs) ===
    "rs",   // Rust
    "ts",   // TypeScript
    "tsx",  // TypeScript + JSX
    "js",   // JavaScript
    "jsx",  // JavaScript + JSX
    "py",   // Python
    "go",   // Go
    "java", // Java
    "c",    // C
    "cpp",  // C++
    "h",    // C/C++ header
    "rb",   // Ruby
    "cs",   // C#
    "kt",   // Kotlin
    "swift",// Swift
    "php",  // PHP
    "svelte", // Svelte
    "vue",  // Vue
    // === Config-as-code (extensão SOULS, não vinha do lean-ctx) ===
    "yaml", // YAML
    "yml",  // YAML (extensão curta)
    "toml", // Cargo.toml, pyproject.toml, etc.
    "json", // package.json, tsconfig.json, etc.
];

/// Diretórios que **nunca** devem ser varridos pelo Harvester ou pelo
/// `tree` MCP. Qualquer entrada aqui é uma porta de exclusão implícita.
///
/// Lista unificada de:
/// - `ctx_heatmap.rs` (dot, node_modules, target, dist, __pycache__, .git)
/// - `search.rs` do SOULS (`.souls_cache`, `.souls_data`, `.cargo`, `.vscode`, `.idea`)
/// - Complementos para cobrir ecossistema Rust/Haskell/Elixir (vendor, build, _build, deps, .venv, __snapshots__)
///
/// **Total: 22 exclusões canônicas.**
pub const EXCLUDE_DIRS: &[&str] = &[
    // === VCS e caches de ferramenta ===
    ".git",            // Git working tree
    ".hg",             // Mercurial
    ".svn",            // Subversion
    ".idea",           // JetBrains IDEs
    ".vscode",         // VS Code settings
    ".cargo",          // Cargo registry cache
    ".souls_cache",    // Cache do SOULS
    ".souls_data",     // SQLite State V5 + heurística (Marco 3.9)
    ".souls_scratchpad", // Scratchpad do SOULS (logs, commits drafts)
    // === Dependências externas (npm/cargo/pip/etc) ===
    "node_modules",    // npm/yarn/pnpm
    "target",          // Cargo build output
    "vendor",          // vendor/ (Rust vendor, Go vendor)
    "deps",            // mix deps (Elixir)
    "build",           // Haskell stack/cabal
    "_build",          // Elixir/Mix
    "dist",            // Frontend builds (vite, webpack)
    "dist-newstyle",   // Cabal v2
    // === Ambientes virtuais e bytecode compilado ===
    "__pycache__",     // Python bytecode cache
    ".venv",           // Python venv
    "venv",            // Python venv (legado)
    ".pytest_cache",   // pytest cache
    // === Snapshots de teste (Rust) ===
    "__snapshots__",   // Insta snapshots
];

/// `true` se a extensão `ext` (sem o ponto) é uma extensão de código-fonte.
///
/// **Performance:** `SOURCE_EXTENSIONS` tem 21 entradas, então um
/// `linear_search` é O(21) ≈ O(1) para os padrões de uso. Se a lista
/// crescer para >100, considere um `phf` (perfect hash function).
pub fn is_source_ext(ext: &str) -> bool {
    SOURCE_EXTENSIONS.contains(&ext)
}

/// `true` se o nome do diretório (não o path completo) deve ser excluído
/// de qualquer varredura.
///
/// Aceita tanto `"target"` quanto `".target"` (com ponto) para tolerar
/// convenções distintas. Dotfiles comuns (`.git`, `.idea`, etc.) são
/// cobertos pela lista canônica.
pub fn is_excluded_dir(name: &str) -> bool {
    if name.starts_with('.') {
        let stripped = name.trim_start_matches('.');
        if EXCLUDE_DIRS.contains(&stripped) || EXCLUDE_DIRS.contains(&name) {
            return true;
        }
    }
    EXCLUDE_DIRS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_source_ext_covers_lean_ctx_list() {
        // As 17 extensões do lean-ctx devem estar presentes.
        for ext in [
            "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h", "rb", "cs", "kt",
            "swift", "php", "svelte", "vue",
        ] {
            assert!(
                is_source_ext(ext),
                "{ext} deve ser reconhecido como source"
            );
        }
    }

    #[test]
    fn is_source_ext_covers_config_as_code() {
        for ext in ["yaml", "yml", "toml", "json"] {
            assert!(
                is_source_ext(ext),
                "{ext} deve ser reconhecido como source"
            );
        }
    }

    #[test]
    fn is_source_ext_rejects_non_code() {
        for ext in ["md", "txt", "log", "png", "jpg", "exe", "dll", "so"] {
            assert!(
                !is_source_ext(ext),
                "{ext} não deve ser source: {SOURCE_EXTENSIONS:?}"
            );
        }
    }

    #[test]
    fn is_excluded_dir_covers_lean_ctx_list() {
        for d in ["node_modules", "target", "dist", "__pycache__"] {
            assert!(is_excluded_dir(d), "{d} deve ser excluído");
        }
    }

    #[test]
    fn is_excluded_dir_handles_dotfiles() {
        for d in [".git", ".idea", ".vscode", ".cargo"] {
            assert!(is_excluded_dir(d), "{d} deve ser excluído (dotfile)");
        }
    }

    #[test]
    fn is_excluded_dir_covers_souls_extras() {
        for d in [".souls_cache", ".souls_data", ".souls_scratchpad", "vendor", "_build", "deps", "build", ".venv"] {
            assert!(is_excluded_dir(d), "{d} deve ser excluído (souls/extra)");
        }
    }

    #[test]
    fn is_excluded_dir_does_not_overexclude() {
        // Pastas legítimas de código não devem ser excluídas.
        for d in ["src", "tests", "examples", "docs", "scripts", "bin", "lib"] {
            assert!(!is_excluded_dir(d), "{d} NÃO deve ser excluído");
        }
    }

    #[test]
    fn constants_have_expected_size() {
        // Invariantes de tamanho — protege contra drift silencioso.
        assert_eq!(SOURCE_EXTENSIONS.len(), 22, "22 extensões canônicas");
        assert_eq!(EXCLUDE_DIRS.len(), 22, "22 exclusões canônicas");
    }
}
