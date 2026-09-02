use std::path::{Path, PathBuf};
use regex::Regex;
use tracing::info;

use crate::harvester::router::StaticAnalysisBlade;
use super::{SastExecutionTarget, SoulsHealthIssue, DiscoveredManifest, ManifestKind, descendant_roots_for_manifest, derive_repo_relative_clean_targets, push_issue, sort_and_dedup_issues};

pub fn is_elixir_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "ex" | "exs"))
        .unwrap_or(false)
}

pub fn sobelow_args_for_root(root: &str) -> Vec<String> {
    vec![
        "sobelow".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "--private".to_string(),
        "--root".to_string(),
        root.to_string(),
    ]
}

pub fn derive_elixir_execution_targets(
    manifests: &[DiscoveredManifest],
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    manifests
        .iter()
        .filter(|manifest| manifest.kind == ManifestKind::MixExs)
        .filter_map(|manifest| {
            let boundaries =
                descendant_roots_for_manifest(manifests, &manifest.execution_root, ManifestKind::MixExs);
            let productive_elixir_files = derive_repo_relative_clean_targets(
                &manifest.execution_root,
                clean_files,
                &boundaries,
                is_elixir_supported_file,
            );
            if productive_elixir_files.is_empty() {
                info!(
                    manifest = %manifest.manifest_path.display(),
                    scope = %manifest.scope,
                    "SAST sobelow: manifesto ignorado por ausencia de codigo Elixir produtivo"
                );
                return None;
            }

            Some(SastExecutionTarget {
                blade: StaticAnalysisBlade::Sobelow,
                execution_root: manifest.execution_root.clone(),
                scope: manifest.scope.clone(),
                scan_targets: vec![".".to_string()],
                command_args: Some(sobelow_args_for_root(".")),
                forced_channel: None,
            })
        })
        .collect()
}

pub fn normalize_sobelow_text_issues(
    repo_path: &Path,
    execution_root: &Path,
    text: &str,
) -> Vec<SoulsHealthIssue> {
    let Ok(finding_re) = Regex::new(
        r#"%\{file: "(?P<file>[^"]+)", line: (?P<line>\d+), type: "(?P<kind>[^"]+)"(?:, variable: (?P<variable>"[^"]+"|:[^,}]+|[A-Za-z_][A-Za-z0-9_]*))?\}"#,
    ) else {
        return Vec::new();
    };

    let mut issues = Vec::new();
    for captures in finding_re.captures_iter(text) {
        let file = captures.name("file").map(|value| value.as_str()).unwrap_or("");
        let line = captures.name("line").map(|value| value.as_str()).unwrap_or("0");
        let kind = captures.name("kind").map(|value| value.as_str()).unwrap_or("Sobelow finding");
        let variable = captures
            .name("variable")
            .map(|value| value.as_str().trim_matches('"'))
            .unwrap_or("");
        let message = if variable.is_empty() {
            format!("{kind} (line {line})")
        } else {
            format!("{kind}: {variable} (line {line})")
        };
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::Sobelow,
            "warning",
            file,
            &message,
        );
    }
    sort_and_dedup_issues(&mut issues);
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::sast::discover_monorepo_manifests;
    use crate::harvester::sast::SandboxExecutor;
    use crate::harvester::sast::test_utils::{MockExecutor, canonicalize_or_self};

    #[test]
    fn test_sobelow_blade_uses_mix_with_private_json_flags() {
        let (binary, args) = crate::harvester::sast::blade_command(StaticAnalysisBlade::Sobelow, &[".".to_string()], None);
        assert_eq!(binary, "mix");
        assert_eq!(args, vec!["sobelow", "--format", "json", "--private", "--root", "."]);
    }

    #[test]
    fn test_sobelow_empty_payload_degrades_without_parse_error() {
        let repo_path = Path::new("C:/repos/example");
        let issues =
            crate::harvester::sast::normalize_sast_output(repo_path, repo_path, StaticAnalysisBlade::Sobelow, b"").unwrap();
        assert!(issues.is_empty());
    }

    #[test]
    fn test_normalize_sobelow_text_fallback_extracts_findings() {
        let repo_path = Path::new("C:/repos/jido");
        let payload = r#"** (UndefinedFunctionError) function Jason.encode!/2 is undefined (module Jason is not available)
    Jason.encode!(%{findings: %{high_confidence: [%{file: "C:/repos/jido/lib/jido/storage/redis.ex", line: 337, type: "Misc.BinToTerm: Unsafe `binary_to_term`", variable: :binary}], low_confidence: [%{file: "C:/repos/jido/lib/jido/plugin/instance.ex", line: 123, type: "DOS.StringToAtom: Unsafe `String.to_atom`", variable: "base_key and as_alias"}], medium_confidence: []}, sobelow_version: "0.14.1", total_findings: 2}, [pretty: true])"#;

        let issues = normalize_sobelow_text_issues(repo_path, repo_path, payload);
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].file, "lib/jido/plugin/instance.ex");
        assert!(issues[0].message.contains("String.to_atom"));
        assert_eq!(issues[1].file, "lib/jido/storage/redis.ex");
        assert!(issues[1].message.contains("binary_to_term"));
    }

    #[test]
    fn test_derive_elixir_execution_targets_set_explicit_root_and_skip_empty_apps() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("mix.exs", "defmodule Root.MixProject do end\n");
        executor.write_repo_file("apps/web/mix.exs", "defmodule Web.MixProject do end\n");
        executor.write_repo_file("apps/web/lib/web/router.ex", "defmodule Web.Router do end\n");
        let repo_root = executor.repo_path().canonicalize().unwrap_or_else(|_| executor.repo_path().to_path_buf());
        let clean_files = vec![canonicalize_or_self(
            repo_root.join("apps/web/lib/web/router.ex"),
        )];

        let manifests = discover_monorepo_manifests(&repo_root);
        let targets = derive_elixir_execution_targets(&manifests, &clean_files);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].scope, "apps/web");
        assert_eq!(targets[0].scan_targets, vec![".".to_string()]);
        assert_eq!(
            targets[0].command_args.as_ref(),
            Some(&sobelow_args_for_root("."))
        );
    }
}
