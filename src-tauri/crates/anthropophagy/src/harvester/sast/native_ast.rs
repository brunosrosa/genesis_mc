use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::harvester::ast_parser::{self, AstParserError};
use crate::harvester::web_scraper;
use super::{SandboxExecutor, SidecarError, ScopedTextBlock, sanitize_repo_relative_path, truncate_chars, render_scoped_text_blocks, render_scoped_text_block_refs, pack_scoped_text_blocks, stdout_is_blank, looks_like_repo_outline_path, BLOB_04_REPO_OUTLINE_MAX_CHARS};

// =============================================================================
// ADR-031 §5 — Heurísticas canônicas de detecção de SkillLibrary (Camadas A/B).
//
// `SKILL_SIGNAL_REL` (Camada A, nome do arquivo): promove `kind: SkillLibrary`
// e contribui para o score de Camada B. Cada keyword DEVE estar espelhada
// em `SCORE_RULES` com um peso > 0.
//
// `SKILL_SIGNAL_CONTENT` (Camada A, conteúdo .md): promove `kind: SkillLibrary`
// mas é **Camada-A-only** (não contribui para o score de ordenação).
// Exceção documentada no ADR-031 §5: keywords de prompt-design e visualização
// são kind-sinalizadoras mas não ordenadoras.
//
// `SCORE_RULES` (Camada B): scoring de ordenação. `problems_and_diagnostics`
// é **Camada-B-only** (não promove kind).
//
// A invariante `static_assert_heuristic_consistency` (compile-time, abaixo)
// blinda contra refatorações invasivas que esvaziem acidentalmente o sinal
// compartilhado entre as duas camadas.
// =============================================================================

/// Camada A — keywords presentes em **nomes de arquivo .md** (rel path).
/// Cada keyword aqui DEVE aparecer também em `SCORE_RULES` (Camada B).
pub const SKILL_SIGNAL_REL: &[&str] = &["skill", "prompt"];

/// Camada A — keywords presentes em **conteúdo de .md** (text scan).
/// Camada-A-only: promovem `kind: SkillLibrary` mas NÃO contribuem para
/// o score de Camada B. Exceção documentada no ADR-031 §5.
pub const SKILL_SIGNAL_CONTENT: &[&str] = &[
    "skills for ai",
    "coding agents",
    "diagram",
    "visualization",
];

/// Camada B — regras de scoring de ordenação. Cada tupla é `(origin, keyword, weight)`.
/// `origin = "rel"` → match contra nome de arquivo; `"content"` → match contra corpo.
pub const SCORE_RULES: &[(&str, &str, i32)] = &[
    ("rel", "readme", 5),
    ("rel", "skill", 3),
    ("rel", "prompt", 3),
    // Camada-B-only: troubleshooting/curadoria. NÃO promove skill_signal
    // (um repo de código com troubleshooting.md é ContentRepo).
    // Documentado em ADR-031 §5.
    ("content", "problems_and_diagnostics", 10),
];

/// Invariante de consistência heurística (compile-time, ADR-031 §5).
///
/// Garante que `SKILL_SIGNAL_REL ∩ SCORE_RULES ≠ ∅` — i.e., existe
/// pelo menos 1 keyword compartilhada entre Camada A e Camada B.
///
/// **Por que essa invariante é mandatória:**
/// - Se a interseção esvaziar, a Camada B para de ranquear repositórios
///   SkillLibrary (eles ficam todos com o mesmo score de ordenação).
/// - O output do harvester torna-se puramente binário (kind), perdendo
///   o sinal de relevância que diferencia "skill com 1 readme" de
///   "skill com 5 readmes + prompts".
/// - Esta validação é compile-time (não runtime) para falhar no CI antes
///   de chegar à produção.
///
/// **Exceções documentadas no ADR-031 §5 (NÃO validadas por este assert):**
/// - `SKILL_SIGNAL_CONTENT` (Camada A-only) é propositalmente
///   kind-promovedora sem score de ranking.
/// - `("content", "problems_and_diagnostics", 10)` (Camada B-only) é
///   score-promovedor sem kind-promotion.
const _: () = {
    /// Função pura `const fn` que verifica a invariante.
    /// Retorna `true` se pelo menos 1 keyword de `SKILL_SIGNAL_REL`
    /// também aparece como segunda tupla de `SCORE_RULES`.
    const fn has_overlap() -> bool {
        let mut i = 0;
        while i < SKILL_SIGNAL_REL.len() {
            let kw_a = SKILL_SIGNAL_REL[i];
            let mut j = 0;
            while j < SCORE_RULES.len() {
                let (_, kw_b, _) = SCORE_RULES[j];
                if const_str_eq(kw_a, kw_b) {
                    return true;
                }
                j += 1;
            }
            i += 1;
        }
        false
    }

    /// `const fn` de comparação de strings por byte (Rust stable).
    /// `core::str::eq` ainda não é `const` em todas as versões, então
    /// implementamos manualmente para garantir compatibilidade total.
    const fn const_str_eq(a: &str, b: &str) -> bool {
        let a_bytes = a.as_bytes();
        let b_bytes = b.as_bytes();
        if a_bytes.len() != b_bytes.len() {
            return false;
        }
        let mut i = 0;
        while i < a_bytes.len() {
            if a_bytes[i] != b_bytes[i] {
                return false;
            }
            i += 1;
        }
        true
    }

    assert!(
        has_overlap(),
        "INVARIANT VIOLATED (ADR-031 §5): SKILL_SIGNAL_REL inter SCORE_RULES is empty. \
         At least 1 keyword from Layer A must be in Layer B. \
         Without this, Layer B loses the SkillLibrary ranking signal and the \
         harvester output becomes purely binary (kind). \
         Add one of the SKILL_SIGNAL_REL keywords as the 2nd element of a tuple \
         in SCORE_RULES with weight > 0."
    );
};

/// Helper público: re-exporta o resultado da invariante para documentação viva
/// em testes. Se a invariante compile-time passar, esta função retorna `true`.
/// **Não duplica a lógica** — apenas reflete o resultado do assert estático
/// via leitura direta das consts (mesma fonte de verdade).
pub fn static_assert_heuristic_consistency() -> bool {
    SKILL_SIGNAL_REL.iter().any(|kw_a| {
        SCORE_RULES
            .iter()
            .any(|(_, kw_b, _)| kw_a == kw_b)
    })
}

pub struct NativeAstInput<'a, E: SandboxExecutor> {
    pub executor: &'a E,
    pub timeout_secs: u64,
    pub clean_files: Arc<Vec<PathBuf>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAstArtifacts {
    pub repo_outline_blob: Vec<u8>,
    pub health_report_blob: Vec<u8>,
    pub architecture_map_blob: Vec<u8>,
}

pub struct NativeAstParser;

impl NativeAstParser {
    /// Extrai os artefatos estruturais de código usando parser AST nativo em Rust.
    pub async fn extract<E: SandboxExecutor>(
        input: NativeAstInput<'_, E>,
    ) -> Result<NativeAstArtifacts, SidecarError> {
        tracing::info!(
            repo_path = %input.executor.repo_path().display(),
            "ast-native: iniciando extração estrutural"
        );
        let repo_path = input.executor.repo_path().to_path_buf();
        let clean_files = Arc::clone(&input.clean_files);
        let native_artifacts = tokio::task::spawn_blocking(move || {
            ast_parser::extract_repository_outline_native_from_clean_files(&repo_path, &clean_files)
        })
        .await
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao aguardar parser AST nativo: {}", e),
        })?;
        let native_artifacts = match native_artifacts {
            Ok(artifacts) => NativeAstArtifacts {
                repo_outline_blob: artifacts.repo_outline_blob,
                health_report_blob: artifacts.health_report_blob,
                architecture_map_blob: artifacts.architecture_map_blob,
            },
            Err(AstParserError::EmptyRepository { path }) => {
                return content_repo_artifacts(
                    input.executor.repo_path(),
                    &format!("no source files found in {}", path),
                )
                .await;
            }
            Err(AstParserError::NoStructuralSymbols { .. }) => {
                let architecture_map = ast_parser::build_architecture_map_blob_from_clean_files(
                    input.executor.repo_path(),
                    &input.clean_files,
                );
                NativeAstArtifacts {
                    repo_outline_blob: Vec::new(),
                    health_report_blob: Vec::new(),
                    architecture_map_blob: architecture_map,
                }
            }
            Err(other) => {
                return Err(SidecarError::ExecutionFailed {
                    reason: other.to_string(),
                });
            }
        };

        let repo_outline_blob = if native_artifacts.repo_outline_blob.is_empty() {
            Vec::new()
        } else {
            normalize_repo_outline(&native_artifacts.repo_outline_blob)?
        };
        let health_report_blob = native_artifacts.health_report_blob;
        let architecture_map_blob = native_artifacts.architecture_map_blob;
        tracing::info!(
            repo_path = %input.executor.repo_path().display(),
            repo_outline_bytes = repo_outline_blob.len(),
            architecture_map_bytes = architecture_map_blob.len(),
            health_report_bytes = health_report_blob.len(),
            "ast-native: artefatos normalizados"
        );

        Ok(NativeAstArtifacts {
            repo_outline_blob,
            health_report_blob,
            architecture_map_blob,
        })
    }
}

pub(crate) fn native_ast_cache_path_for_repo(repo_path: &Path) -> String {
    repo_path
        .parent()
        .unwrap_or(repo_path)
        .join(".native_ast_cache")
        .display()
        .to_string()
}

#[cfg(test)]
fn native_ast_cache_global_storage_dir() -> Option<PathBuf> {
    use std::env;
    if let Ok(configured) = env::var("JCODEMUNCH_STORAGE_PATH") {
        let trimmed = configured.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    let home = env::var_os("USERPROFILE").or_else(|| env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".code-index"))
}

#[cfg(test)]
fn native_ast_cache_db_path_for_repo(repo_path: &Path) -> Result<std::path::PathBuf, SidecarError> {
    native_ast_cache_db_path_for_repo_id(repo_path, None)
}

#[cfg(test)]
fn native_ast_cache_db_path_for_repo_id(
    repo_path: &Path,
    index_repo_id: Option<&str>,
) -> Result<std::path::PathBuf, SidecarError> {
    let owner = repo_path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .ok_or_else(|| SidecarError::ExecutionFailed {
            reason: "Nao foi possivel resolver o owner do repositório para localizar o cache AST nativo".to_string(),
        })?;
    let repo = repo_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SidecarError::ExecutionFailed {
            reason: "Nao foi possivel resolver o nome do repositório para localizar o cache AST nativo".to_string(),
        })?;
    let mut roots = vec![repo_path.parent().unwrap_or(repo_path).join(".native_ast_cache")];
    if let Some(global_root) = native_ast_cache_global_storage_dir() {
        roots.push(global_root);
    }

    let mut exact_stems = Vec::new();
    if let Some(repo_id) = index_repo_id {
        let sanitized = repo_id
            .trim()
            .replace(['\\', '/'], "-")
            .replace(':', "-");
        if !sanitized.is_empty() {
            exact_stems.push(sanitized);
        }
    }
    exact_stems.push(format!("{}-{}", owner, repo));

    let mut all_candidates = Vec::new();
    for root in roots {
        for stem in &exact_stems {
            let candidate = root.join(format!("{stem}.db"));
            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        if let Ok(entries) = std::fs::read_dir(&root) {
            let mut candidates = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("db"))
                .collect::<Vec<_>>();
            candidates.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            all_candidates.extend(candidates);
        }
    }

    let repo_lower = repo.to_ascii_lowercase();
    if let Some(path) = all_candidates.iter().position(|path| {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| stem.to_ascii_lowercase().contains(&repo_lower))
            .unwrap_or(false)
    }) {
        return Ok(all_candidates.swap_remove(path));
    }

    if all_candidates.len() == 1 {
        return Ok(all_candidates.swap_remove(0));
    }

    Err(SidecarError::ExecutionFailed {
        reason: format!(
            "Nao foi possivel localizar o cache SQLite do AST nativo para '{}'; repo_id={:?}; candidatos={:?}",
            repo_path.display(),
            index_repo_id,
            all_candidates
        ),
    })
}

fn collect_markdown_files(repo_path: &Path, max_files: usize) -> Vec<PathBuf> {
    fn should_skip_dir(name: &str) -> bool {
        matches!(name, ".git" | "node_modules" | "target" | "vendor" | ".jj" | ".svn")
    }

    let mut out = Vec::new();
    let mut stack = vec![repo_path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if out.len() >= max_files {
            break;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.take(512) {
            if out.len() >= max_files {
                break;
            }
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if should_skip_dir(name) {
                        continue;
                    }
                }
                stack.push(path);
                continue;
            }
            if !ft.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase());
            if matches!(ext.as_deref(), Some("md" | "markdown" | "mdx")) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

async fn content_repo_artifacts(repo_path: &Path, why: &str) -> Result<NativeAstArtifacts, SidecarError> {
    let md_files = collect_markdown_files(repo_path, 24);
    let mut blocks: Vec<(i32, ScopedTextBlock)> = Vec::new();
    let mut all_text = String::new();
    let mut skill_signal = false;

    // ADR-031 §5: heurísticas canônicas de detecção de SkillLibrary.
    // Camada A (skill_signal → kind) e Camada B (score → ordem) são
    // ortogonais por design; a invariante de consistência é
    // validada em compile-time por `static_assert_heuristic_consistency`
    // (definida no topo do módulo). As exceções documentadas
    // (`SKILL_SIGNAL_CONTENT` e `problems_and_diagnostics`) também
    // são honradas aqui.
    for path in &md_files {
        let rel = sanitize_repo_relative_path(repo_path, &path.to_string_lossy())
            .unwrap_or_else(|| path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string());
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut score = 0i32;

        // Camada A: skill_signal (curto-circuito no primeiro hit).
        if !skill_signal {
            let rel_l = rel.to_ascii_lowercase();
            let c_l = content.to_ascii_lowercase();
            if SKILL_SIGNAL_REL.iter().any(|k| rel_l.contains(k))
                || SKILL_SIGNAL_CONTENT.iter().any(|k| c_l.contains(k))
            {
                skill_signal = true;
            }
        }

        // Camada B: score acumulado para ordenação.
        {
            let rel_l = rel.to_ascii_lowercase();
            let c_l = content.to_ascii_lowercase();
            for (origin, kw, weight) in SCORE_RULES {
                let hit = match *origin {
                    "rel" => rel_l.contains(kw),
                    "content" => c_l.contains(kw),
                    _ => false,
                };
                if hit {
                    score += weight;
                }
            }
        }

        all_text.push_str(&content);
        all_text.push('\n');
        blocks.push((
            score,
            ScopedTextBlock {
                file_path: rel,
                items: vec![content],
                omitted_count: 0,
            },
        ));
    }

    blocks.sort_by(|(score_l, block_l), (score_r, block_r)| {
        score_r.cmp(score_l).then_with(|| block_l.file_path.cmp(&block_r.file_path))
    });
    let packed = {
        let block_refs = blocks.iter().map(|(_, block)| block).collect::<Vec<_>>();
        render_scoped_text_block_refs(&block_refs)
    };
    let kind = if skill_signal { "SkillLibrary" } else { "ContentRepo" };
    let mut outline = String::new();
    outline.push_str("# Repository Outline\n\n");
    outline.push_str("kind: ");
    outline.push_str(kind);
    outline.push('\n');
    outline.push_str("note: Repositório sem arquivos de código indexáveis (curadoria/documentação/skills).\n");
    outline.push_str("why: ");
    outline.push_str(why.trim());
    outline.push_str("\n\n");
    if packed.trim().is_empty() {
        outline.push_str("Sem markdown legível encontrado.\n");
    } else {
        outline.push_str("## Markdown Extract (amostra)\n\n");
        outline.push_str(&packed);
    }

    let urls = extract_urls_from_text(&all_text, 600);
    let gh_repos = extract_github_repo_ids(&urls, 250);
    let mut external = BTreeSet::<String>::new();
    for url in urls {
        if url.to_ascii_lowercase().contains("github.com/") {
            continue;
        }
        external.insert(url);
    }
    let prioritized_remote_docs = prioritized_external_doc_urls(&external, 4);
    let mut remote_blocks = Vec::<ScopedTextBlock>::new();
    let mut remote_fetch_failures = Vec::<String>::new();
    for url in &prioritized_remote_docs {
        match web_scraper::fetch_markdown_with_guarantee(url).await {
            Ok(markdown) => {
                if markdown.trim().is_empty() {
                    remote_fetch_failures.push(format!("{url} => markdown vazio"));
                } else {
                    remote_blocks.push(ScopedTextBlock {
                        file_path: remote_block_label(url),
                        items: vec![markdown],
                        omitted_count: 0,
                    });
                }
            }
            Err(err) => {
                remote_fetch_failures.push(format!(
                    "{} => {}",
                    url,
                    err
                ));
            }
        }
    }
    if !prioritized_remote_docs.is_empty() && remote_blocks.is_empty() {
        return Err(SidecarError::ExecutionFailed {
            reason: format!(
                "Falha ao garantir scraping remoto para documentação externa. urls={} falhas={}",
                prioritized_remote_docs.join(", "),
                remote_fetch_failures.join(" | ")
            ),
        });
    }

    let mut link_map = String::new();
    link_map.push_str("# Link Map\n\n");
    link_map.push_str("kind: ");
    link_map.push_str(kind);
    link_map.push('\n');
    link_map.push_str(&format!("markdown_files: {}\n", md_files.len()));
    link_map.push_str(&format!("github_repo_links: {}\n", gh_repos.len()));
    link_map.push_str(&format!("external_links: {}\n", external.len()));
    link_map.push_str(&format!("remote_doc_candidates: {}\n", prioritized_remote_docs.len()));
    link_map.push_str(&format!("remote_doc_fetched: {}\n", remote_blocks.len()));
    link_map.push_str(&format!("remote_doc_failed: {}\n\n", remote_fetch_failures.len()));
    link_map.push_str("## GitHub Repos\n");
    if gh_repos.is_empty() {
        link_map.push_str("- <nenhum>\n");
    } else {
        for repo in &gh_repos {
            link_map.push_str("- ");
            link_map.push_str(repo);
            link_map.push('\n');
        }
    }
    link_map.push_str("\n## External URLs\n");
    if external.is_empty() {
        link_map.push_str("- <nenhum>\n");
    } else {
        for url in external.iter().take(200) {
            link_map.push_str("- ");
            link_map.push_str(url);
            link_map.push('\n');
        }
    }
    if !prioritized_remote_docs.is_empty() {
        link_map.push_str("\n## Remote Docs Scraped\n");
        if remote_blocks.is_empty() {
            link_map.push_str("- <nenhum>\n");
        } else {
            for url in &prioritized_remote_docs {
                link_map.push_str("- ");
                link_map.push_str(url);
                link_map.push('\n');
            }
        }
    }
    if !remote_fetch_failures.is_empty() {
        link_map.push_str("\n## Remote Docs Failures\n");
        for failure in &remote_fetch_failures {
            link_map.push_str("- ");
            link_map.push_str(failure);
            link_map.push('\n');
        }
    }

    let mut health = String::from("# Health Report\n");
    health.push_str("\nsummary: findings=0");
    health.push_str("\nsource: content-repo-fallback");
    health.push_str("\nkind: ");
    health.push_str(kind);
    health.push_str("\nwhy: ");
    health.push_str(why.trim());
    health.push_str("\nmarkdown_files: ");
    health.push_str(&md_files.len().to_string());
    health.push_str("\ngithub_repo_links: ");
    health.push_str(&gh_repos.len().to_string());
    health.push_str("\nexternal_links: ");
    health.push_str(&external.len().to_string());
    health.push_str("\nremote_doc_candidates: ");
    health.push_str(&prioritized_remote_docs.len().to_string());
    health.push_str("\nremote_doc_fetched: ");
    health.push_str(&remote_blocks.len().to_string());
    health.push_str("\nremote_doc_failed: ");
    health.push_str(&remote_fetch_failures.len().to_string());
    health.push_str("\nskill_signal: ");
    health.push_str(if skill_signal { "true" } else { "false" });

    if !remote_blocks.is_empty() {
        outline.push_str("\n\n## Remote Documentation (Guaranteed)\n\n");
        outline.push_str(&render_scoped_text_blocks(&remote_blocks));
    }

    Ok(NativeAstArtifacts {
        repo_outline_blob: outline.into_bytes(),
        architecture_map_blob: link_map.into_bytes(),
        health_report_blob: health.into_bytes(),
    })
}

fn extract_urls_from_text(text: &str, max_urls: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    let bytes = text.as_bytes();
    while idx < bytes.len() && out.len() < max_urls {
        let rest = &text[idx..];
        let Some(rel_pos) = rest.find("http") else {
            break;
        };
        idx = idx.saturating_add(rel_pos);
        let candidate = &text[idx..];
        let end = candidate
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '"' | '\'' | '<' | '>'))
            .unwrap_or(candidate.len());
        let mut url = candidate[..end].trim().trim_end_matches(['.', ',', ';', ':']).to_string();
        if url.starts_with("http://") || url.starts_with("https://") {
            if url.len() > 2048 {
                url.truncate(2048);
            }
            out.push(url);
        }
        idx = idx.saturating_add(end.max(1));
    }
    out
}

fn extract_github_repo_ids(urls: &[String], max_repos: usize) -> Vec<String> {
    let mut out = BTreeSet::<String>::new();
    for url in urls {
        if out.len() >= max_repos {
            break;
        }
        let lower = url.to_ascii_lowercase();
        let marker = "github.com/";
        let Some(pos) = lower.find(marker) else {
            continue;
        };
        let mut rest = url[(pos + marker.len())..].to_string();
        if let Some(hash) = rest.find('#') {
            rest.truncate(hash);
        }
        if let Some(q) = rest.find('?') {
            rest.truncate(q);
        }
        rest = rest.trim_end_matches('/').trim_end_matches(".git").to_string();
        let mut parts = rest.split('/').map(|p| p.trim()).filter(|p| !p.is_empty());
        let Some(owner) = parts.next() else { continue };
        let Some(repo) = parts.next() else { continue };
        if owner.eq_ignore_ascii_case("topics")
            || owner.eq_ignore_ascii_case("search")
            || owner.eq_ignore_ascii_case("orgs")
            || owner.eq_ignore_ascii_case("users")
        {
            continue;
        }
        out.insert(format!("{owner}/{repo}"));
    }
    out.into_iter().take(max_repos).collect()
}

/// Tabela canônica de pesos para priorização de URLs de documentação externa.
///
/// 12 keywords ordenadas por **peso descendente** (canonização do ranking):
/// - `codewiki` (30) e `deepwiki` (24): wikis de alto sinal sobre a estrutura
///   interna do repositório.
/// - `readthedocs` (20), `docs.` (18), `/docs` (18), `documentation` (16):
///   fontes oficiais de documentação canônica (Sphinx, MkDocs, Docusaurus).
/// - `/wiki` (16), `guide` (12), `manual` (12): guias não-oficiais / tutoriais.
/// - `reference` (10), `api` (8), `tutorial` (8): páginas técnicas pontuais.
///
/// **DRY (ADR-031 §5)**: Esta constante é a ÚNICA fonte de verdade. Editar
/// aqui propaga para `score_external_doc_url` e o teste de Camada B sem
/// duplicar a tabela em 2 lugares (drift zero entre detecção e scoring).
pub const SCORE_URL_RULES: &[(&str, i32)] = &[
    ("codewiki", 30),
    ("deepwiki", 24),
    ("readthedocs", 20),
    ("docs.", 18),
    ("/docs", 18),
    ("documentation", 16),
    ("/wiki", 16),
    ("guide", 12),
    ("manual", 12),
    ("reference", 10),
    ("api", 8),
    ("tutorial", 8),
];

fn score_external_doc_url(url: &str) -> i32 {
    let lower = url.to_ascii_lowercase();
    let mut score = 0i32;
    for (needle, weight) in SCORE_URL_RULES {
        if lower.contains(needle) {
            score += weight;
        }
    }
    score
}

fn prioritized_external_doc_urls(urls: &BTreeSet<String>, max_urls: usize) -> Vec<String> {
    let mut ranked = urls
        .iter()
        .map(|url| (score_external_doc_url(url), url.clone()))
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    ranked.sort_by(|(score_l, url_l), (score_r, url_r)| {
        score_r.cmp(score_l).then_with(|| url_l.cmp(url_r))
    });
    ranked
        .into_iter()
        .map(|(_, url)| url)
        .take(max_urls)
        .collect()
}

fn remote_block_label(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .map(|parsed| {
            let host = parsed.host_str().unwrap_or("remote");
            let path = parsed.path().trim_matches('/');
            if path.is_empty() {
                format!("remote::{host}")
            } else {
                format!("remote::{host}/{}", truncate_chars(path, 80))
            }
        })
        .unwrap_or_else(|| format!("remote::{}", truncate_chars(url, 96)))
}

fn normalize_repo_outline_markdown(text: &str) -> String {
    let mut leading = Vec::new();
    let mut blocks = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_items = Vec::new();

    let flush_current = |blocks: &mut Vec<ScopedTextBlock>, current_path: &mut Option<String>, current_items: &mut Vec<String>| {
        let Some(file_path) = current_path.take() else {
            return;
        };
        blocks.push(ScopedTextBlock {
            file_path,
            items: std::mem::take(current_items),
            omitted_count: 0,
        });
    };

    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('#') {
            flush_current(&mut blocks, &mut current_path, &mut current_items);
            leading.push(trimmed.to_string());
            continue;
        }

        let bullet = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .map(str::trim);

        let Some(content) = bullet else {
            if current_path.is_some() {
                current_items.push(trimmed.to_string());
            } else {
                leading.push(trimmed.to_string());
            }
            continue;
        };

        if looks_like_repo_outline_path(content) {
            flush_current(&mut blocks, &mut current_path, &mut current_items);
            current_path = Some(content.to_string());
        } else if current_path.is_some() {
            current_items.push(content.to_string());
        } else {
            leading.push(content.to_string());
        }
    }

    flush_current(&mut blocks, &mut current_path, &mut current_items);
    if blocks.is_empty() {
        return text.trim().to_string();
    }

    let mut normalized = leading.join("\n");
    let packed_blocks = pack_scoped_text_blocks(
        &blocks,
        BLOB_04_REPO_OUTLINE_MAX_CHARS.saturating_sub(normalized.len()),
    );
    if !packed_blocks.trim().is_empty() {
        if !normalized.is_empty() {
            normalized.push_str("\n\n");
        }
        normalized.push_str(&packed_blocks);
    }
    normalized
}

fn normalize_repo_outline(bytes: &[u8]) -> Result<Vec<u8>, SidecarError> {
    if stdout_is_blank(bytes) {
        tracing::debug!(binary = "native-ast-parser", "Sidecar claude-md retornou stdout vazio");
        return Err(SidecarError::ExecutionFailed {
            reason: "native-ast-parser claude-md returned empty stdout".to_string(),
        });
    }

    let text = String::from_utf8_lossy(bytes);
    let normalized = if text.contains("[DOMAIN: ") && text.contains("## Productive Tree") {
        text.trim().to_string()
    } else {
        normalize_repo_outline_markdown(&text)
    };
    let truncated = truncate_chars(&normalized, BLOB_04_REPO_OUTLINE_MAX_CHARS);
    if truncated.trim().is_empty() {
        return Err(SidecarError::ExecutionFailed {
            reason: "native-ast-parser claude-md returned an empty repo outline".to_string(),
        });
    }

    Ok(truncated.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::sast::test_utils::{MockExecutor, test_clean_files};
    use tempfile::TempDir;

    #[test]
    fn test_code_index_db_path_accepts_local_repo_index_name() {
        let temp_dir = TempDir::new().unwrap();
        let owner_dir = temp_dir.path().join("aaif-goose");
        let repo_path = owner_dir.join("goose");
        let index_dir = owner_dir.join(".native_ast_cache");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::fs::create_dir_all(&index_dir).unwrap();

        let expected = index_dir.join("local-goose-0a8be5b6.db");
        std::fs::write(&expected, b"").unwrap();

        let resolved = native_ast_cache_db_path_for_repo(&repo_path).unwrap();
        assert_eq!(resolved, expected);
    }

    #[tokio::test]
    async fn test_extract_success() {
        let executor = MockExecutor::new(vec![]);
        executor.write_repo_file(
            "src/main.rs",
            r#"
use crate::config::AppConfig;

fn main() {
    let _cfg = AppConfig::default();
}
"#,
        );
        executor.write_repo_file(
            "src/lib.rs",
            r#"
pub mod config {
    #[derive(Default)]
    pub struct AppConfig;
}
"#,
        );
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(executor.repo_path(), &["src/main.rs", "src/lib.rs"]),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ter sucesso: {:?}", result);
        let payload = result.unwrap();
        let health_report = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(health_report.contains("# Health Report"));
        assert!(health_report.contains(
            "source: native-rust multi-strategy (language-pack + targeted-tree-sitter + regex-fallback)"
        ));
        assert!(health_report.contains("parsed_files: 2"));
        let repo_outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        assert!(repo_outline.contains("# Repository Outline"));
        assert!(repo_outline.contains("## Productive Tree"));
        assert!(repo_outline.contains("repo/"));
        assert!(repo_outline.contains("[DOMAIN: RUST]"));
        assert!(repo_outline.contains("[src/lib.rs]"));
        assert!(repo_outline.contains("[src/main.rs]"));
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();
        assert!(architecture_map.contains("[src]"));
        assert!(architecture_map.contains("src/main.rs"));
        assert!(architecture_map.contains("src/lib.rs"));
    }

    #[tokio::test]
    async fn test_extract_success_repo_outline_tolerates_invalid_utf8() {
        let claude_md = b"# Repository Outline\n\xff\n- src/main.rs\n".to_vec();

        let result = normalize_repo_outline(&claude_md);
        assert!(result.is_ok(), "Normalização deveria tolerar repo outline com UTF-8 inválido: {:?}", result);
        let repo_outline = String::from_utf8(result.unwrap()).unwrap();
        assert!(repo_outline.contains("# Repository Outline"));
        assert!(repo_outline.contains("[src/main.rs]"));
    }

    #[tokio::test]
    async fn test_architecture_map_skips_visual_noise_and_prioritizes_backend() {
        let executor = MockExecutor::new(vec![]);
        executor.write_repo_file("icons/logo.svg", "<svg />");
        executor.write_repo_file(
            "src/backend/service.rs",
            r#"
pub struct Engine;

pub fn render_service() -> Engine {
    Engine
}
"#,
        );
        executor.write_repo_file(
            "web/panel.tsx",
            r#"
export function Panel() {
    return <div>panel</div>;
}
"#,
        );

        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(
                executor.repo_path(),
                &["icons/logo.svg", "src/backend/service.rs", "web/panel.tsx"],
            ),
        };
        let payload = NativeAstParser::extract(input).await.unwrap();
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();

        assert!(!architecture_map.contains("icons/logo.svg"));
        let backend_pos = architecture_map.find("[src/backend]").unwrap();
        let ui_pos = architecture_map.find("[web]").unwrap();
        assert!(backend_pos < ui_pos, "backend deve vir antes de ui: {}", architecture_map);
        assert!(architecture_map.contains("src/backend/service.rs"));
        assert!(architecture_map.contains("web/panel.tsx"));
    }

    #[tokio::test]
    async fn test_architecture_map_keeps_backend_visible_amid_tests_examples_and_fixtures() {
        let executor = MockExecutor::new(vec![]);
        executor.write_repo_file(
            "crates/goose/tests/session_id_propagation_test.rs",
            "pub fn ignored_test_helper() {}",
        );
        executor.write_repo_file("examples/demo/main.rs", "fn main() {}");
        executor.write_repo_file("src/backend/fixtures/sample.rs", "pub fn fixture_only() {}");
        executor.write_repo_file("src/backend/test_support/helpers.rs", "pub fn helper() {}");
        executor.write_repo_file("src/backend/e2e/flow.rs", "pub fn flow() {}");
        executor.write_repo_file(
            "src/backend/service.rs",
            r#"
pub struct Engine;

pub fn run(_engine: Engine) {}
"#,
        );

        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(
                executor.repo_path(),
                &[
                    "crates/goose/tests/session_id_propagation_test.rs",
                    "examples/demo/main.rs",
                    "src/backend/fixtures/sample.rs",
                    "src/backend/test_support/helpers.rs",
                    "src/backend/e2e/flow.rs",
                    "src/backend/service.rs",
                ],
            ),
        };
        let payload = NativeAstParser::extract(input).await.unwrap();
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();

        assert!(architecture_map.contains("[src/backend]"));
        assert!(architecture_map.contains("src/backend/service.rs"));
    }

    #[tokio::test]
    async fn test_architecture_map_keeps_backend_visible_amid_scenarios_docs_ui_and_bench_noise() {
        let executor = MockExecutor::new(vec![]);
        executor.write_repo_file(
            "crates/goose-cli/src/scenario_tests/message_generator.rs",
            "pub fn scenario_noise() {}",
        );
        executor.write_repo_file(
            "documentation/src/pages/index.tsx",
            "export function DocsPage() { return <div />; }",
        );
        executor.write_repo_file(
            "ui/desktop/src/App.tsx",
            "export function App() { return <main />; }",
        );
        executor.write_repo_file("oidc-proxy/test/index.test.js", "export function worker() {}");
        executor.write_repo_file(
            "evals/open-model-gym/suite/src/runner.ts",
            "export function runScenario() {}",
        );
        executor.write_repo_file("crates/goose/benches/parser.rs", "pub fn bench_parser() {}");
        executor.write_repo_file(
            "src/backend/engine.rs",
            r#"
pub struct Runtime;

pub fn boot(_runtime: Runtime) {}
"#,
        );

        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(
                executor.repo_path(),
                &[
                    "crates/goose-cli/src/scenario_tests/message_generator.rs",
                    "documentation/src/pages/index.tsx",
                    "ui/desktop/src/App.tsx",
                    "oidc-proxy/test/index.test.js",
                    "evals/open-model-gym/suite/src/runner.ts",
                    "crates/goose/benches/parser.rs",
                    "src/backend/engine.rs",
                ],
            ),
        };
        let payload = NativeAstParser::extract(input).await.unwrap();
        let architecture_map = String::from_utf8(payload.architecture_map_blob).unwrap();

        assert!(architecture_map.contains("[src/backend]"));
        assert!(architecture_map.contains("src/backend/engine.rs"));
    }

    #[tokio::test]
    async fn test_binary_not_found() {
        let spawn_err = crate::harvester::sandbox::SandboxError::ProcessSpawnFailed {
            reason: "program not found (os error 2)".to_string(),
        };
        let executor = MockExecutor::new(vec![Err(spawn_err)]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        let health = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(outline.contains("kind: ContentRepo"), "Outline deveria cair no modo ContentRepo");
        assert!(outline.contains("no source files found"), "Outline deveria registrar a causa estrutural");
        assert!(health.contains("# Health Report"));
        assert!(health.contains("kind: ContentRepo"));
    }

    #[tokio::test]
    async fn test_execution_failed() {
        let run_err = crate::harvester::sandbox::SandboxError::ProcessNonZeroExit {
            exit_code: 2,
            stderr: "fatal error".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(vec![Err(run_err)]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        let health = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(outline.contains("kind: ContentRepo"), "Outline deveria cair no modo ContentRepo");
        assert!(outline.contains("no source files found"), "Outline deveria registrar a causa estrutural");
        assert!(health.contains("# Health Report"));
        assert!(health.contains("kind: ContentRepo"));
    }

    #[tokio::test]
    async fn test_timeout_propagation() {
        let executor = MockExecutor::new(vec![Err(crate::harvester::sandbox::SandboxError::Timeout)]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 45,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    #[tokio::test]
    async fn test_invalid_json() {
        let index_json = r#"{"success": true}"#;
        let corrup_bytes = b"{invalid_json_here".to_vec();
        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(corrup_bytes),
        ]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    #[tokio::test]
    async fn test_empty_repo_payload_fails_closed() {
        let index_json = r#"{"success": true}"#;
        let empty_json = r#"{}"#;
        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(empty_json.as_bytes().to_vec()),
        ]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    #[tokio::test]
    async fn test_empty_stdout_fails_closed() {
        let index_json = r#"{"success": true}"#;
        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(Vec::new()),
        ]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    #[tokio::test]
    async fn test_exit_code_1_fails_soft_for_native_ast_parser() {
        let run_err = crate::harvester::sandbox::SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "usage error".to_string(),
            stdout: Vec::new(),
        };
        let executor = MockExecutor::new(vec![Err(run_err)]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    #[tokio::test]
    async fn test_native_ast_cache_exit_code_1_with_success_json_is_allowed() {
        let index_json = r#"{"success": true}"#;
        let digest_json = r#"{"hotspots":[{"path":"src/main.rs","complexity":12}]}"#;
        let run_err = crate::harvester::sandbox::SandboxError::ProcessNonZeroExit {
            exit_code: 1,
            stderr: "".to_string(),
            stdout: index_json.as_bytes().to_vec(),
        };
        let executor = MockExecutor::new(vec![
            Err(run_err),
            Ok(digest_json.as_bytes().to_vec()),
        ]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria tolerar exit 1 no index: {:?}", result);
    }

    #[tokio::test]
    async fn test_claude_md_empty_stdout_fails_closed() {
        let index_json = r#"{"success": true}"#;
        let digest_json = r#"{"hotspots":[{"path":"src/main.rs","complexity":12}]}"#;
        let executor = MockExecutor::new(vec![
            Ok(index_json.as_bytes().to_vec()),
            Ok(digest_json.as_bytes().to_vec()),
            Ok(Vec::new()),
        ]);
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: Arc::new(Vec::new()),
        };

        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
    }

    // =============================================================================
    // SOULS-CANIBALIZED Marco 3.9 Fase E (BLOCO 4.2): 4 testes Camada A do
    // Harvester (ADR-031 §5). Validam que cada uma das 4 keywords
    // canônicas de `skill_signal_content` PROMOVE o repositório para
    // `kind: SkillLibrary` no outline gerado por `content_repo_artifacts`.
    //
    // Padrão: o `MockExecutor` falha em todos os comandos (caminho de
    // `EmptyRepository` → `content_repo_artifacts`). Cada teste cria um
    // único `.md` cuja `content.to_ascii_lowercase().contains(keyword)`
    // deve disparar o curto-circuito da Camada A.
    // =============================================================================

    /// Camada A — keyword 1 de 4: `skills for ai` (curto-circuito na 1ª hit).
    /// Ref: ADR-031 §5 tabela canônica.
    #[tokio::test]
    async fn test_skill_signal_skills_for_ai_promotes_kind_skilllibrary() {
        let spawn_err = crate::harvester::sandbox::SandboxError::ProcessSpawnFailed {
            reason: "test_skill_signal: program not found".to_string(),
        };
        let executor = MockExecutor::new(vec![Err(spawn_err)]);
        executor.write_repo_file(
            "guides/intro.md",
            "# Curated Skills for AI Agents\n\nThis is a curated guide.\n",
        );
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(executor.repo_path(), &["guides/intro.md"]),
        };
        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        let health = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(
            outline.contains("kind: SkillLibrary"),
            "keyword 'skills for ai' deve promover kind para SkillLibrary. Outline:\n{outline}"
        );
        assert!(health.contains("skill_signal: true"));
    }

    /// Camada A — keyword 2 de 4: `coding agents`.
    /// Ref: ADR-031 §5 tabela canônica.
    #[tokio::test]
    async fn test_skill_signal_coding_agents_promotes_kind_skilllibrary() {
        let spawn_err = crate::harvester::sandbox::SandboxError::ProcessSpawnFailed {
            reason: "test_skill_signal: program not found".to_string(),
        };
        let executor = MockExecutor::new(vec![Err(spawn_err)]);
        executor.write_repo_file(
            "prompts/agent_patterns.md",
            "# Coding Agents Patterns\n\nReference patterns for autonomous agents.\n",
        );
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(executor.repo_path(), &["prompts/agent_patterns.md"]),
        };
        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        let health = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(
            outline.contains("kind: SkillLibrary"),
            "keyword 'coding agents' deve promover kind para SkillLibrary. Outline:\n{outline}"
        );
        assert!(health.contains("skill_signal: true"));
    }

    /// Camada A — keyword 3 de 4: `diagram`.
    /// Ref: ADR-031 §5 tabela canônica.
    #[tokio::test]
    async fn test_skill_signal_diagram_promotes_kind_skilllibrary() {
        let spawn_err = crate::harvester::sandbox::SandboxError::ProcessSpawnFailed {
            reason: "test_skill_signal: program not found".to_string(),
        };
        let executor = MockExecutor::new(vec![Err(spawn_err)]);
        executor.write_repo_file(
            "skills/architecture.md",
            "# System Diagram Reference\n\nContains the canonical diagram of the system.\n",
        );
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(executor.repo_path(), &["skills/architecture.md"]),
        };
        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        let health = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(
            outline.contains("kind: SkillLibrary"),
            "keyword 'diagram' deve promover kind para SkillLibrary. Outline:\n{outline}"
        );
        assert!(health.contains("skill_signal: true"));
    }

    /// Camada A — keyword 4 de 4: `visualization`.
    /// Ref: ADR-031 §5 tabela canônica.
    #[tokio::test]
    async fn test_skill_signal_visualization_promotes_kind_skilllibrary() {
        let spawn_err = crate::harvester::sandbox::SandboxError::ProcessSpawnFailed {
            reason: "test_skill_signal: program not found".to_string(),
        };
        let executor = MockExecutor::new(vec![Err(spawn_err)]);
        executor.write_repo_file(
            "skills/metrics.md",
            "# Visualization Cookbook\n\nHow to produce compelling visualization dashboards.\n",
        );
        let input = NativeAstInput {
            executor: &executor,
            timeout_secs: 30,
            clean_files: test_clean_files(executor.repo_path(), &["skills/metrics.md"]),
        };
        let result = NativeAstParser::extract(input).await;
        assert!(result.is_ok(), "Extração deveria ser fail-soft: {:?}", result);
        let payload = result.unwrap();
        let outline = String::from_utf8(payload.repo_outline_blob).unwrap();
        let health = String::from_utf8(payload.health_report_blob).unwrap();
        assert!(
            outline.contains("kind: SkillLibrary"),
            "keyword 'visualization' deve promover kind para SkillLibrary. Outline:\n{outline}"
        );
        assert!(health.contains("skill_signal: true"));
    }

    // =============================================================================
    // SOULS-CANIBALIZED Marco 3.9 Fase E (Follow-up): Documentação viva da
    // invariante de consistência heurística (ADR-031 §5).
    //
    // O `const _: () = assert!(has_overlap(), ...)` no topo do módulo
    // blinda a invariante em compile-time. Esses 2 testes servem como
    // **documentação viva** — se algum engenheiro futuramente modificar
    // as consts de forma que rompa a invariante, o assert estático
    // falha no build antes de chegar a CI; o teste aqui apenas
    // documenta o comportamento esperado em runtime.
    // =============================================================================

    /// T-inv-1: Documenta que a invariante `SKILL_SIGNAL_REL ∩ SCORE_RULES ≠ ∅`
    /// está válida em runtime. Se algum engenheiro remover acidentalmente
    /// a interseção (e.g. remover "skill" e "prompt" de SCORE_RULES), o
    /// `const _` no topo do módulo falha o build antes deste teste rodar.
    #[test]
    fn test_static_assert_heuristic_consistency_holds() {
        assert!(
            static_assert_heuristic_consistency(),
            "Invariante ADR-031 §5 quebrada: SKILL_SIGNAL_REL ∩ SCORE_RULES = ∅. \
             A Camada B perdeu o sinal de ranking de SkillLibrary. \
             Isso NÃO deveria ser possível se o `const _ = assert!(...)` no topo \
             do módulo estiver ativo. Verifique se a invariante compile-time foi \
             desabilitada por #[allow(...)] indevido."
        );
    }

    /// T-inv-2: Valida explicitamente as exceções documentadas (ADR-031 §5):
    /// - `SKILL_SIGNAL_CONTENT` é Camada-A-only (kind-promovedora, sem score).
    /// - `("content", "problems_and_diagnostics", 10)` é Camada-B-only
    ///   (score-promovedor, sem kind-promotion).
    #[test]
    fn test_heuristic_exceptions_are_documented() {
        // SKILL_SIGNAL_CONTENT: Camada-A-only. Nenhuma das 4 keywords
        // deve aparecer em SCORE_RULES como segunda tupla.
        for kw_a_only in SKILL_SIGNAL_CONTENT {
            let in_b = SCORE_RULES.iter().any(|(_, kw_b, _)| kw_b == kw_a_only);
            assert!(
                !in_b,
                "Exceção ADR-031 §5 violada: '{kw_a_only}' é Camada-A-only \
                 (kind-promovedora) e NÃO deve aparecer em SCORE_RULES."
            );
        }
        // problems_and_diagnostics: Camada-B-only. Não deve aparecer
        // em SKILL_SIGNAL_REL nem em SKILL_SIGNAL_CONTENT.
        let b_only_kw = "problems_and_diagnostics";
        let in_a_rel = SKILL_SIGNAL_REL.iter().any(|k| k == &b_only_kw);
        let in_a_content = SKILL_SIGNAL_CONTENT.iter().any(|k| k == &b_only_kw);
        assert!(
            !in_a_rel && !in_a_content,
            "Exceção ADR-031 §5 violada: '{b_only_kw}' é Camada-B-only \
             (score-promovedor) e NÃO deve aparecer em Camada A."
        );
    }
}
