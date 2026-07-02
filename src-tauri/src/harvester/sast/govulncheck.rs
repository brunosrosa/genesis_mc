use std::path::{Path, PathBuf};
use tracing::info;

use crate::harvester::router::StaticAnalysisBlade;
use super::{SidecarError, SastExecutionTarget, SodaHealthIssue, DiscoveredManifest, ManifestKind, descendant_roots_for_manifest, derive_repo_relative_clean_targets, is_go_supported_file, push_issue, sort_and_dedup_issues};

pub fn govulncheck_args_for_module() -> Vec<String> {
    vec![
        "-format".to_string(),
        "json".to_string(),
        "./...".to_string(),
    ]
}

pub fn derive_go_execution_targets(
    manifests: &[DiscoveredManifest],
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    manifests
        .iter()
        .filter(|manifest| manifest.kind == ManifestKind::GoMod)
        .filter_map(|manifest| {
            let boundaries =
                descendant_roots_for_manifest(manifests, &manifest.execution_root, ManifestKind::GoMod);
            let productive_go_files = derive_repo_relative_clean_targets(
                &manifest.execution_root,
                clean_files,
                &boundaries,
                is_go_supported_file,
            );
            if productive_go_files.is_empty() {
                info!(
                    manifest = %manifest.manifest_path.display(),
                    scope = %manifest.scope,
                    "SAST govulncheck: manifesto ignorado por ausencia de pacotes Go produtivos"
                );
                return None;
            }

            Some(SastExecutionTarget {
                blade: StaticAnalysisBlade::Govulncheck,
                execution_root: manifest.execution_root.clone(),
                scope: manifest.scope.clone(),
                scan_targets: vec!["./...".to_string()],
                command_args: Some(govulncheck_args_for_module()),
                forced_channel: None,
            })
        })
        .collect()
}

pub fn normalize_govulncheck_output(
    repo_path: &Path,
    execution_root: &Path,
    bytes: &[u8],
) -> Result<Vec<SodaHealthIssue>, SidecarError> {
    let text = String::from_utf8_lossy(bytes);
    let mut issues = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let value = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(finding) = value.get("finding") else {
            continue;
        };
        let osv = finding.get("osv").and_then(|value| value.as_str()).unwrap_or("govulncheck");
        let trace = finding
            .get("trace")
            .and_then(|value| value.as_array())
            .and_then(|trace| trace.first());
        let file = trace
            .and_then(|frame| frame.get("position"))
            .and_then(|position| position.get("filename"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let package = trace
            .and_then(|frame| frame.get("module"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let symbol = trace
            .and_then(|frame| frame.get("function"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let message = if package.is_empty() && symbol.is_empty() {
            format!("{osv}: vulnerabilidade detectada")
        } else {
            format!("{osv}: {}", format!("{package} {symbol}").trim())
        };
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::Govulncheck,
            "error",
            file,
            &message,
        );
    }
    sort_and_dedup_issues(&mut issues);
    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::sast::SandboxExecutor;
    use crate::harvester::sast::test_utils::{MockExecutor, canonicalize_or_self};
    use crate::harvester::sast::discover_monorepo_manifests;

    #[test]
    fn test_derive_go_execution_targets_anchor_govulncheck_to_real_modules_only() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("go.mod", "module root.example\n\ngo 1.22\n");
        executor.write_repo_file("README.md", "docs only\n");
        executor.write_repo_file("services/api/go.mod", "module api.example\n\ngo 1.22\n");
        executor.write_repo_file("services/api/cmd/api/main.go", "package main\nfunc main() {}\n");
        executor.write_repo_file("tools/empty/go.mod", "module empty.example\n\ngo 1.22\n");
        let repo_root = executor.repo_path().canonicalize().unwrap_or_else(|_| executor.repo_path().to_path_buf());
        let clean_files = vec![
            canonicalize_or_self(repo_root.join("README.md")),
            canonicalize_or_self(repo_root.join("services/api/cmd/api/main.go")),
        ];

        let manifests = discover_monorepo_manifests(&repo_root);
        let targets = derive_go_execution_targets(&manifests, &clean_files);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].scope, "services/api");
        assert_eq!(targets[0].scan_targets, vec!["./...".to_string()]);
        assert_eq!(
            targets[0].command_args.as_ref(),
            Some(&govulncheck_args_for_module())
        );
    }
}
