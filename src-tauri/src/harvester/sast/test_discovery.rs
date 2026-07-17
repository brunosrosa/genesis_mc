use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;
use regex::Regex;

use crate::harvester::detect::{SingleStack, StackProfile};
use crate::harvester::repo_radar;
use super::{SidecarError, ScopedTextBlock, cached_regex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestIntentPayload {
    pub runner_name: String,
    pub timed_out: bool,
    pub blocks: Vec<ScopedTextBlock>,
}

pub struct NativeTestDiscoveryInput<'a> {
    pub repo_path: &'a Path,
    pub profile: &'a StackProfile,
}

const UNIVERSAL_TEST_SKIP_SEGMENTS: [&str; 8] = [
    "docs",
    "documentation",
    "examples",
    "mock",
    "mocks",
    "fixtures",
    "test_support",
    "e2e",
];
const UNIVERSAL_TEST_SKIP_SUBSTRINGS: [&str; 3] = ["integration_mocks", "mock_", "/docs/"];
const STATIC_TEST_DISCOVERY_READ_BYTES: usize = 50 * 1024;

pub struct NativeTestDiscoverySidecar;

impl NativeTestDiscoverySidecar {
    pub async fn extract(input: NativeTestDiscoveryInput<'_>) -> Result<TestIntentPayload, SidecarError> {
        let repo_path = input.repo_path.to_path_buf();
        let profile = input.profile.clone();
        let blocks =
            tokio::task::spawn_blocking(move || discover_static_test_entries_bfs(&repo_path, &profile))
                .await
                .map_err(|e| SidecarError::ExecutionFailed {
                    reason: format!("Static test discovery join failed: {}", e),
                })??;
        Ok(TestIntentPayload {
            runner_name: "static-ast-radar".to_string(),
            timed_out: false,
            blocks,
        })
    }
}

fn primary_stack(profile: &StackProfile) -> Option<SingleStack> {
    match profile {
        StackProfile::Rust => Some(SingleStack::Rust),
        StackProfile::CCpp => Some(SingleStack::CCpp),
        StackProfile::Elixir => Some(SingleStack::Elixir),
        StackProfile::NodeJS => Some(SingleStack::NodeJS),
        StackProfile::Python => Some(SingleStack::Python),
        StackProfile::Go => Some(SingleStack::Go),
        StackProfile::JVM => Some(SingleStack::JVM),
        StackProfile::DotNet => Some(SingleStack::DotNet),
        StackProfile::Mixed(stacks) => stacks.first().cloned(),
        StackProfile::Unknown => None,
    }
}

fn should_skip_discovered_test_entry(value: &str) -> bool {
    let normalized = value.trim().replace('\\', "/").to_ascii_lowercase();
    if normalized.is_empty() {
        return true;
    }

    if UNIVERSAL_TEST_SKIP_SUBSTRINGS
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return true;
    }

    normalized
        .split(is_semgrep_path_separator)
        .filter(|part| !part.is_empty())
        .any(|part| UNIVERSAL_TEST_SKIP_SEGMENTS.contains(&part))
}

#[allow(clippy::manual_pattern_char_comparison)]
fn is_semgrep_path_separator(ch: char) -> bool {
    matches!(ch, '/' | ':' | '>' | ' ')
}

fn is_known_test_file_path(value: &str) -> bool {
    let lower = value.trim().replace('\\', "/").to_ascii_lowercase();
    [
        ".test.ts",
        ".test.tsx",
        ".test.js",
        ".test.jsx",
        ".test.mjs",
        ".test.cjs",
        ".test.mts",
        ".test.cts",
        ".spec.ts",
        ".spec.tsx",
        ".spec.js",
        ".spec.jsx",
        ".spec.mjs",
        ".spec.cjs",
        ".spec.mts",
        ".spec.cts",
        "_test.go",
        "_test.py",
        "_test.exs",
        "test_",
        "__tests__",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn is_inline_test_candidate_source_file(profile: &StackProfile, path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("rs") => supports_stack(profile, SingleStack::Rust),
        Some("go") => supports_stack(profile, SingleStack::Go),
        _ => false,
    }
}

fn supports_stack(profile: &StackProfile, target: SingleStack) -> bool {
    match profile {
        StackProfile::Mixed(stacks) => stacks.contains(&target),
        StackProfile::Unknown => true,
        _ => primary_stack(profile) == Some(target),
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    let normalized_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let normalized_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    normalized_path
        .strip_prefix(&normalized_root)
        .unwrap_or(normalized_path.as_path())
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_supported_test_file(profile: &StackProfile, path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    let normalized = path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();

    if is_inline_test_candidate_source_file(profile, path) {
        return true;
    }

    match extension.as_deref() {
        Some("py") => {
            supports_stack(profile, SingleStack::Python)
                && (normalized.contains("/tests/")
                    || normalized.ends_with("_test.py")
                    || path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.to_ascii_lowercase().starts_with("test_"))
                        .unwrap_or(false))
        }
        Some("exs") => {
            supports_stack(profile, SingleStack::Elixir)
                && (normalized.ends_with("_test.exs")
                    || normalized.contains("/test/")
                    || normalized.contains("/tests/"))
        }
        Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts") => {
            supports_stack(profile, SingleStack::NodeJS) && is_known_test_file_path(&normalized)
        }
        _ => false,
    }
}

fn read_static_test_file(path: &Path) -> Result<Option<String>, SidecarError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| SidecarError::ExecutionFailed {
        reason: format!("Falha ao abrir '{}': {}", path.display(), e),
    })?;
    let mut buf = Vec::new();
    let _ = (&mut file)
        .take(STATIC_TEST_DISCOVERY_READ_BYTES as u64)
        .read_to_end(&mut buf)
        .map_err(|e| SidecarError::ExecutionFailed {
            reason: format!("Falha ao ler primeiros {} bytes de '{}': {}", STATIC_TEST_DISCOVERY_READ_BYTES, path.display(), e),
        })?;
    match String::from_utf8(buf) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}

fn compact_signature_text(signature: &str) -> Option<String> {
    let compact = signature
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .trim_end_matches('{')
        .trim()
        .to_string();
    if compact.is_empty() {
        None
    } else {
        Some(compact)
    }
}

fn normalize_rust_test_signature(signature: &str) -> Option<String> {
    let head = signature.split('{').next().unwrap_or(signature).trim();
    let compact = compact_signature_text(head)?;
    let compact = compact.strip_suffix("()").unwrap_or(&compact).trim().to_string();
    if compact.is_empty() {
        None
    } else {
        Some(compact)
    }
}

fn is_rust_test_attribute(trimmed: &str) -> bool {
    trimmed.starts_with("#[")
        && (trimmed.contains("test]") || trimmed.contains("rstest") || trimmed.contains("fixture"))
}

fn extract_python_test_entries_shallow(content: &str) -> Vec<String> {
    static PYTHON_TEST_DEF_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let Some(re) = cached_regex(
        &PYTHON_TEST_DEF_RE,
        r#"(?m)^\s*(?:async\s+def|def)\s+(test_[A-Za-z0-9_]+)\s*\("#,
    ) else {
        return Vec::new();
    };
    let mut entries = BTreeSet::new();
    for captures in re.captures_iter(content) {
        if let Some(name) = captures.get(1) {
            entries.insert(format!("def {}", name.as_str()));
        }
    }
    entries.into_iter().collect()
}

fn extract_rust_test_entries_shallow(content: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    let lines: Vec<&str> = content.lines().take(2_000).collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        let is_test_attr = is_rust_test_attribute(trimmed);
        let is_test_fn = trimmed.starts_with("fn test_")
            || trimmed.starts_with("async fn test_")
            || trimmed.contains(" fn test_");
        if is_test_attr || is_test_fn {
            // A linha da assinatura pode estar 1 linha apos o #[test]
            let sig_line_idx = if is_test_attr && i + 1 < lines.len() {
                i + 1
            } else {
                i
            };
            let sig_trimmed = lines[sig_line_idx].trim();
            let normalized = sig_trimmed.trim().trim_end_matches(';').trim();
            if let Some(signature) = normalize_rust_test_signature(normalized) {
                // PRD-045: anexa o primeiro assert/expect como "significado"
                // do teste, para que LLMs 3-7B entendam o que ele VALIDA
                // sem precisar parsear o codigo.
                let behavior = extract_rust_test_body_assertion(&lines, sig_line_idx + 1);
                let entry = match behavior {
                    Some(b) => format!("{signature}  // {b}"),
                    None => signature,
                };
                out.insert(entry);
            }
            // Pula ate o final do bloco (heuristica: brace_depth).
            // Importante: comecar pela linha da assinatura para que o `{` seja contado.
            i = sig_line_idx;
            let mut brace_depth: i32 = 0;
            while i < lines.len() {
                let l = lines[i];
                brace_depth += l.matches('{').count() as i32;
                brace_depth -= l.matches('}').count() as i32;
                // Termo do bloco: brace_depth voltou a zero E a linha tem `}`
                // (ex: `fn test_x() {}` em uma unica linha).
                if brace_depth <= 0 && l.contains('}') {
                    i += 1;
                    // Se a proxima linha for o inicio de um novo teste (`#[test]`
                    // ou `fn test_`), NAO pular: o while externo precisa
                    // re-detecta-la. Sem isso, perde-se 1 a cada 2 testes
                    // com corpo vazio inline.
                    if i < lines.len() {
                        let next = lines[i].trim();
                        if next.starts_with("#[test")
                            || next.starts_with("#[tokio::test")
                            || next.starts_with("fn test_")
                            || next.starts_with("async fn test_")
                        {
                            // Continua o while externo a partir de i (ja incrementado)
                        }
                    }
                    break;
                }
                if trimmed_is_new_top_level_fn(l) {
                    break;
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out.into_iter().collect()
}

/// Heuristica: linha que parece abrir uma nova funcao top-level.
fn trimmed_is_new_top_level_fn(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("fn ") || t.starts_with("async fn ") || t.starts_with("pub fn ")
}

/// PRD-045: extrai o primeiro assert/expect do corpo de um teste Rust.
/// Retorna "asserts: <macro>" ou "expects: <expr>" para anexar como
/// significado/contrato do teste.
///
/// Lei IV: retorna None se nao encontrar nenhum assert/expect no body
/// (teste vazio, ou apenas side-effects), para nao inventar semantica.
fn extract_rust_test_body_assertion(lines: &[&str], start: usize) -> Option<String> {
    let mut brace_depth: i32 = 0;
    for line in &lines[start..] {
        brace_depth += line.matches('{').count() as i32;
        brace_depth -= line.matches('}').count() as i32;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        // Macro: assert!, assert_eq!, assert_ne!, assert_matches!, ...
        if let Some(rest) = trimmed.strip_prefix("assert") {
            if let Some(after_bang) = rest.find('!') {
                let mac = rest[..after_bang].trim().trim_start_matches('_').trim_end_matches('_');
                if !mac.is_empty()
                    && mac.chars().all(|c| c.is_ascii_alphabetic() || c == '_')
                {
                    let kind = if mac.is_empty() {
                        "assert".to_string()
                    } else {
                        // mac ja vem como "eq", "ne", etc (sem o prefixo "assert_")
                        format!("assert_{mac}")
                    };
                    return Some(format!("asserts: {kind}"));
                }
            }
        }
        // expect("...") - panics with a message
        if trimmed.contains(".expect(") {
            return Some("expects: .expect(...)".to_string());
        }
        // unwrap() - panics if None/Err
        if trimmed.contains(".unwrap()") {
            return Some("expects: .unwrap()".to_string());
        }
        // should_panic (attribute no #[should_panic])
        if trimmed.starts_with("#[should_panic") {
            return Some("asserts: should_panic".to_string());
        }
        // Fim do bloco antes de achar assert
        if brace_depth <= 0 && line.contains('}') {
            return None;
        }
    }
    None
}

fn extract_go_test_entries_shallow(content: &str) -> Vec<String> {
    static GO_TEST_RE: OnceLock<Option<Regex>> = OnceLock::new();
    static GO_SUBTEST_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let Some(test_re) = cached_regex(
        &GO_TEST_RE,
        r#"(?m)^\s*(func\s+(?:\([^)]+\)\s+)?(?:Test|Fuzz)[A-Z][A-Za-z0-9_]*\s*\([^)]*\))"#,
    ) else {
        return Vec::new();
    };
    let Some(subtest_re) = cached_regex(
        &GO_SUBTEST_RE,
        r#"\b[A-Za-z0-9_]+\.(?:Run|Fuzz)\(\s*"([^"]+)""#,
    ) else {
        return Vec::new();
    };
    let mut entries = BTreeSet::new();
    for captures in test_re.captures_iter(content) {
        if let Some(signature) = captures.get(1) {
            if let Some(signature) = compact_signature_text(signature.as_str()) {
                entries.insert(signature);
            }
        }
    }
    for captures in subtest_re.captures_iter(content) {
        if let Some(name) = captures.get(1) {
            entries.insert(format!("subtest \"{}\"", name.as_str()));
        }
    }
    entries.into_iter().collect()
}

fn extract_elixir_test_entries_shallow(content: &str) -> Vec<String> {
    static ELIXIR_TEST_RE: OnceLock<Option<Regex>> = OnceLock::new();
    static ELIXIR_DESCRIBE_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let Some(test_re) = cached_regex(&ELIXIR_TEST_RE, r#"(?m)^\s*test\s+"([^"]+)"\s+do"#) else {
        return Vec::new();
    };
    let Some(describe_re) = cached_regex(
        &ELIXIR_DESCRIBE_RE,
        r#"(?m)^\s*describe\s+"([^"]+)"\s+do"#,
    ) else {
        return Vec::new();
    };
    let mut entries = BTreeSet::new();
    for captures in describe_re.captures_iter(content) {
        if let Some(name) = captures.get(1) {
            entries.insert(format!("describe \"{}\"", name.as_str()));
        }
    }
    for captures in test_re.captures_iter(content) {
        if let Some(name) = captures.get(1) {
            entries.insert(format!("test \"{}\"", name.as_str()));
        }
    }
    entries.into_iter().collect()
}

fn extract_frontend_test_entries_shallow(content: &str) -> Vec<String> {
    static FRONTEND_TEST_RE: OnceLock<Option<Regex>> = OnceLock::new();
    let Some(re) = cached_regex(
        &FRONTEND_TEST_RE,
        r#"(?:^|[^\w])(describe|it|test)(?:\.(?:only|skip|concurrent|each|todo|failing))*\s*\(\s*(?:"([^"\r\n]+)"|'([^'\r\n]+)'|`([^`\r\n]+)`)"#,
    ) else {
        return Vec::new();
    };
    let mut out = BTreeSet::new();
    for captures in re.captures_iter(content) {
        let kind = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
        let label = captures
            .get(2)
            .or_else(|| captures.get(3))
            .or_else(|| captures.get(4))
            .map(|m| m.as_str())
            .unwrap_or_default()
            .trim();
        if !kind.is_empty() && !label.is_empty() {
            out.insert(format!(r#"{kind} "{label}""#));
        }
    }
    out.into_iter().collect()
}



fn discover_static_test_entries_bfs(
    repo_path: &Path,
    profile: &StackProfile,
) -> Result<Vec<ScopedTextBlock>, SidecarError> {
    let mut blocks = Vec::new();
    let radar = repo_radar::build_repo_radar(repo_path);
    for path in radar.all_files() {
        if !is_supported_test_file(profile, path) {
            continue;
        }

        let relative = relative_display(repo_path, path);
        if should_skip_discovered_test_entry(&relative) {
            continue;
        }

        let Some(content) = read_static_test_file(path)? else {
            continue;
        };
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());

        let discovered = match extension.as_deref() {
            Some("rs") => extract_rust_test_entries_shallow(&content),
            Some("py") => extract_python_test_entries_shallow(&content),
            Some("go") => extract_go_test_entries_shallow(&content),
            Some("exs") => extract_elixir_test_entries_shallow(&content),
            Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts") => {
                extract_frontend_test_entries_shallow(&content)
            }
            _ => Vec::new(),
        };

        let mut items = Vec::new();
        for entry in discovered {
            let candidate = format!("{} :: {}", relative, entry);
            if !should_skip_discovered_test_entry(&candidate) {
                items.push(entry);
            }
        }
        if items.is_empty() {
            continue;
        }

        blocks.push(ScopedTextBlock {
            file_path: relative,
            items,
            omitted_count: 0,
        });
    }

    Ok(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_native_test_discovery_uses_static_ast_for_rust() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::create_dir_all(dir.path().join("docs")).unwrap();
        std::fs::create_dir_all(dir.path().join("mock")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            "#[cfg(test)]\nmod tests {\n    #[tokio::test]\n    async fn test_domain_logic_stays() {}\n}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("docs/test_docs.rs"),
            "#[test]\nfn test_docs_should_not_enter_blob() {}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("mock/test_mock.rs"),
            "#[test]\nfn test_mock_should_be_ignored() {}\n",
        )
        .unwrap();
        let profile = StackProfile::Rust;

        let payload = NativeTestDiscoverySidecar::extract(NativeTestDiscoveryInput {
            repo_path: dir.path(),
            profile: &profile,
        })
        .await
        .unwrap();

        assert_eq!(payload.runner_name, "static-ast-radar");
        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "src/lib.rs"
                && block.items.contains(&"async fn test_domain_logic_stays".to_string())));
        assert!(!payload.blocks.iter().any(|block| block.file_path.contains("docs")));
        assert!(!payload.blocks.iter().any(|block| block.file_path.contains("mock")));
    }

    #[tokio::test]
    async fn test_native_test_discovery_preserves_all_items_per_file() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let mut content = String::new();
        for index in 0..8 {
            content.push_str(&format!("#[test]\nfn test_case_{index}() {{}}\n"));
        }
        std::fs::write(dir.path().join("src/lib.rs"), content).unwrap();

        let payload = NativeTestDiscoverySidecar::extract(NativeTestDiscoveryInput {
            repo_path: dir.path(),
            profile: &StackProfile::Rust,
        })
        .await
        .unwrap();

        assert_eq!(payload.blocks.len(), 1);
        assert_eq!(payload.blocks[0].file_path, "src/lib.rs");
        assert_eq!(payload.blocks[0].items.len(), 8);
        assert_eq!(payload.blocks[0].omitted_count, 0);
        assert!(payload.blocks[0].items.contains(&"fn test_case_7".to_string()));
    }

    #[tokio::test]
    async fn test_native_test_discovery_detects_go_python_elixir_and_frontend_intent() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("go")).unwrap();
        std::fs::create_dir_all(dir.path().join("python")).unwrap();
        std::fs::create_dir_all(dir.path().join("elixir")).unwrap();
        std::fs::create_dir_all(dir.path().join("web")).unwrap();

        std::fs::write(
            dir.path().join("go/math_test.go"),
            r#"
package demo

import "testing"

func TestSum(t *testing.T) {
    t.Run("adds positives", func(t *testing.T) {})
}
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("python/test_api.py"),
            r#"
def helper():
    return 1

async def test_async_healthcheck():
    assert True

def test_sync_healthcheck():
    assert True
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("elixir/user_test.exs"),
            r#"
defmodule Demo.UserTest do
  use ExUnit.Case

  describe "create_user/1" do
    test "persists valid payload" do
      assert true
    end
  end
end
"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("web/login.spec.ts"),
            r#"
describe("login flow", () => {
  it("renders button", () => {});
  test.skip("shows errors", () => {});
});
"#,
        )
        .unwrap();

        let payload = NativeTestDiscoverySidecar::extract(NativeTestDiscoveryInput {
            repo_path: dir.path(),
            profile: &StackProfile::Mixed(vec![
                SingleStack::Go,
                SingleStack::Python,
                SingleStack::Elixir,
                SingleStack::NodeJS,
            ]),
        })
        .await
        .unwrap();

        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "go/math_test.go"
                && block.items.contains(&"func TestSum(t *testing.T)".to_string())
                && block.items.contains(&r#"subtest "adds positives""#.to_string())));
        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "python/test_api.py"
                && block.items.contains(&"def test_async_healthcheck".to_string())
                && block.items.contains(&"def test_sync_healthcheck".to_string())));
        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "elixir/user_test.exs"
                && block.items.contains(&r#"describe "create_user/1""#.to_string())
                && block.items.contains(&r#"test "persists valid payload""#.to_string())));
        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "web/login.spec.ts"
                && block.items.contains(&r#"describe "login flow""#.to_string())
                && block.items.contains(&r#"it "renders button""#.to_string())
                && block.items.contains(&r#"test "shows errors""#.to_string())));
    }

    #[tokio::test]
    async fn test_native_test_discovery_detects_inline_go_tests_outside_test_dirs() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("pkg")).unwrap();
        std::fs::write(
            dir.path().join("pkg/smoke.go"),
            r#"
package demo

import "testing"

func helper() {}

func TestSmokePath(t *testing.T) {
    if t == nil {
        panic("unreachable")
    }
}
"#,
        )
        .unwrap();

        let payload = NativeTestDiscoverySidecar::extract(NativeTestDiscoveryInput {
            repo_path: dir.path(),
            profile: &StackProfile::Go,
        })
        .await
        .unwrap();

        assert!(payload
            .blocks
            .iter()
            .any(|block| block.file_path == "pkg/smoke.go"
                && block.items.contains(&"func TestSmokePath(t *testing.T)".to_string())
                && !block.items.iter().any(|item| item.contains("panic"))));
    }

    // PRD-045: testes de extracao de "significado" do teste (assert/expect).
    // O objetivo eh que LLMs 3-7B consigam inferir o QUE o teste valida
    // sem precisar parsear o codigo.

    #[test]
    fn test_extract_rust_test_body_assertion_finds_assert_eq() {
        let lines = vec![
            "fn test_addition() {",
            "    let result = 2 + 2;",
            "    assert_eq!(result, 4);",
            "}",
        ];
        let result = extract_rust_test_body_assertion(&lines, 1);
        assert_eq!(result, Some("asserts: assert_eq".to_string()));
    }

    #[test]
    fn test_extract_rust_test_body_assertion_finds_expect() {
        let lines = vec![
            "fn test_parse() {",
            "    let val: i32 = \"42\".parse().expect(\"not a number\");",
            "    assert_eq!(val, 42);",
            "}",
        ];
        let result = extract_rust_test_body_assertion(&lines, 1);
        // expect() vem antes de assert, deve pegar expect primeiro
        assert_eq!(result, Some("expects: .expect(...)".to_string()));
    }

    #[test]
    fn test_extract_rust_test_body_assertion_finds_unwrap() {
        let lines = vec![
            "fn test_open() {",
            "    let f = File::open(\"x.txt\").unwrap();",
            "}",
        ];
        let result = extract_rust_test_body_assertion(&lines, 1);
        assert_eq!(result, Some("expects: .unwrap()".to_string()));
    }

    #[test]
    fn test_extract_rust_test_body_assertion_finds_should_panic() {
        let lines = vec![
            "#[should_panic]",
            "fn test_panic() {",
            "    panic!(\"oh no\");",
            "}",
        ];
        let result = extract_rust_test_body_assertion(&lines, 0);
        assert_eq!(result, Some("asserts: should_panic".to_string()));
    }

    #[test]
    fn test_extract_rust_test_body_assertion_returns_none_for_empty_test() {
        let lines = vec![
            "fn test_empty() {",
            "    let _ = setup();",
            "}",
        ];
        let result = extract_rust_test_body_assertion(&lines, 1);
        // Nenhum assert/expect => None (Lei IV)
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_rust_test_entries_attach_assertion_context() {
        // Teste de integracao: extract_rust_test_entries_shallow deve
        // anexar o contexto do assert como // expects: <kind>
        let content = "\
#[test]
fn test_addition() {
    let result = 2 + 2;
    assert_eq!(result, 4);
}

#[test]
fn test_unwrap_path() {
    let x = parse(\"5\").unwrap();
    assert_eq!(x, 5);
}
";
        let entries = extract_rust_test_entries_shallow(content);
        assert_eq!(entries.len(), 2);
        // O primeiro teste tem // expects: assert_eq
        let addition = entries.iter().find(|e| e.contains("test_addition")).unwrap();
        assert!(addition.contains("// asserts: assert_eq"), "Esperava contexto do assert, got: {addition}");
        // O segundo teste tem // expects: .unwrap() (vem antes do assert)
        let unwrap = entries.iter().find(|e| e.contains("test_unwrap_path")).unwrap();
        assert!(unwrap.contains("// expects: .unwrap()"), "Esperava contexto do unwrap, got: {unwrap}");
    }
}
