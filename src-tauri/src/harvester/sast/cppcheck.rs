use std::path::{Path, PathBuf};
use regex::Regex;

use crate::harvester::router::StaticAnalysisBlade;
use super::{SidecarError, SastExecutionTarget, SoulsHealthIssue, push_issue, sort_and_dedup_issues, extract_xml_payload, is_cpp_supported_file, derive_repo_relative_clean_targets, CPPCHECK_FILE_LIST_CHUNK_SIZE, blade_file_batch_scope};

pub fn cppcheck_args(scan_targets: &[String]) -> Vec<String> {
    cppcheck_args_for_targets(scan_targets)
}

pub fn cppcheck_args_for_targets(scan_targets: &[String]) -> Vec<String> {
    let mut args = vec![
        "--xml".to_string(),
        "--xml-version=2".to_string(),
        "--enable=warning".to_string(),
        "--disable=style,performance,portability,information".to_string(),
        "--inline-suppr".to_string(),
        "--suppress=missingInclude".to_string(),
        "--suppress=unusedFunction".to_string(),
        "--suppress=unmatchedSuppression".to_string(),
    ];
    if scan_targets.is_empty() {
        args.push(".".to_string());
    } else {
        args.extend(scan_targets.iter().cloned());
    }
    args
}

pub fn derive_cppcheck_execution_targets(
    repo_path: &Path,
    clean_files: &[PathBuf],
) -> Vec<SastExecutionTarget> {
    let repo_root = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let scan_targets = derive_repo_relative_clean_targets(&repo_root, clean_files, &[], is_cpp_supported_file);
    if scan_targets.is_empty() {
        return Vec::new();
    }

    scan_targets
        .chunks(CPPCHECK_FILE_LIST_CHUNK_SIZE)
        .enumerate()
        .map(|(idx, chunk)| {
            let chunk_targets = chunk.to_vec();
            SastExecutionTarget {
                blade: StaticAnalysisBlade::Cppcheck,
                execution_root: repo_root.clone(),
                scope: blade_file_batch_scope(".", idx + 1),
                scan_targets: chunk_targets.clone(),
                command_args: Some(cppcheck_args_for_targets(&chunk_targets)),
                forced_channel: None,
            }
        })
        .collect()
}

pub fn normalize_cppcheck_output(
    repo_path: &Path,
    execution_root: &Path,
    bytes: &[u8],
) -> Result<Vec<SoulsHealthIssue>, SidecarError> {
    let mut issues = Vec::new();
    let payload = match extract_xml_payload(bytes) {
        Some(payload) => payload,
        None => {
            let unstructured = String::from_utf8_lossy(bytes);
            if unstructured.trim().is_empty() {
                push_issue(
                    &mut issues,
                    repo_path,
                    execution_root,
                    StaticAnalysisBlade::Cppcheck,
                    "info",
                    "",
                    "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck.",
                );
                return Ok(issues);
            }
            push_issue(
                &mut issues,
                repo_path,
                execution_root,
                StaticAnalysisBlade::Cppcheck,
                "warning",
                "",
                &format!("cppcheck output nao estruturado preservado: {unstructured}"),
            );
            return Ok(issues);
        }
    };

    let text = String::from_utf8_lossy(payload);
    let opt_errors = text.find("<errors>").and_then(|start| {
        text.find("</errors>")
            .map(|end| &text[start..(end + "</errors>".len())])
    });

    let Some(errors_block) = opt_errors else {
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::Cppcheck,
            "info",
            "",
            "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck.",
        );
        return Ok(issues);
    };

    let error_re = Regex::new(r#"<error\s+id="([^"]+)"\s+severity="([^"]+)"\s+msg="([^"]+)"(?:>|\s[^>]*>)"#).unwrap();
    let location_re = Regex::new(r#"<location\s+file="([^"]+)"\s+line="([^"]+)"\s*/>"#).unwrap();

    let mut start_idx = 0;
    while let Some(pos) = errors_block[start_idx..].find("<error ") {
        let abs_pos = start_idx + pos;
        let end_pos = errors_block[abs_pos..]
            .find("</error>")
            .map(|e| abs_pos + e + "</error>".len())
            .unwrap_or_else(|| {
                errors_block[abs_pos..]
                    .find("/>")
                    .map(|e| abs_pos + e + "/>".len())
                    .unwrap_or(errors_block.len())
            });
        let block = &errors_block[abs_pos..end_pos];
        start_idx = end_pos;

        let Some(error_caps) = error_re.captures(block) else {
            continue;
        };
        let id = error_caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
        let severity = error_caps.get(2).map(|m| m.as_str()).unwrap_or("warning");
        let msg = error_caps.get(3).map(|m| m.as_str()).unwrap_or("cppcheck issue");

        let mut file = "";
        let mut line = "";
        if let Some(loc_caps) = location_re.captures(block) {
            file = loc_caps.get(1).map(|m| m.as_str()).unwrap_or("");
            line = loc_caps.get(2).map(|m| m.as_str()).unwrap_or("");
        }

        let full_msg = if line.is_empty() {
            format!("[{id}] {msg}")
        } else {
            format!("L{line}: [{id}] {msg}")
        };

        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::Cppcheck,
            severity,
            file,
            &full_msg,
        );
    }

    if issues.is_empty() {
        push_issue(
            &mut issues,
            repo_path,
            execution_root,
            StaticAnalysisBlade::Cppcheck,
            "info",
            "",
            "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck.",
        );
    } else {
        sort_and_dedup_issues(&mut issues);
    }

    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::sast::SandboxExecutor;
    use crate::harvester::sast::test_utils::{MockExecutor, canonicalize_or_self};

    #[test]
    fn test_cppcheck_blade_enforces_xml_v2_args() {
        let (binary, args) = crate::harvester::sast::blade_command(StaticAnalysisBlade::Cppcheck, &[".".to_string()], None);
        assert_eq!(binary, "cppcheck");
        assert!(args.iter().any(|arg| arg == "--xml"));
        assert!(args.iter().any(|arg| arg == "--xml-version=2"));
        assert!(!args.iter().any(|arg| arg.starts_with("--error-exitcode")));
    }

    #[test]
    fn test_cppcheck_args_are_security_scoped() {
        let (binary, args) = crate::harvester::sast::blade_command(StaticAnalysisBlade::Cppcheck, &[".".to_string()], None);
        assert_eq!(binary, "cppcheck");
        assert!(args.iter().any(|arg| arg == "--enable=warning"));
        assert!(args
            .iter()
            .any(|arg| arg == "--disable=style,performance,portability,information"));
        assert!(!args.iter().any(|arg| arg == "--enable=all"));
    }

    #[test]
    fn test_normalize_cppcheck_output_ignores_progress_prefix() {
        let repo_path = Path::new("C:/repos/example");
        let payload = concat!(
            "Checking src\\\\main.c ...\r\n",
            "1/1 files checked 100% done\r\n",
            "<results><errors><error id=\"memleak\" severity=\"warning\" msg=\"Memory leak: ptr\">",
            "<location file=\"src/main.c\" line=\"42\"/></error></errors></results>"
        );

        let issues = normalize_cppcheck_output(repo_path, repo_path, payload.as_bytes()).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].file, "src/main.c");
        assert!(issues[0].message.contains("memleak"));
    }

    #[test]
    fn test_normalize_cppcheck_output_accepts_empty_results_as_clean_info() {
        let repo_path = Path::new("C:/repos/example");
        let payload = r#"<?xml version="1.0"?><results></results>"#;

        let issues = normalize_cppcheck_output(repo_path, repo_path, payload.as_bytes()).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, "info");
        assert_eq!(issues[0].file, "");
        assert_eq!(
            issues[0].message,
            "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck."
        );
        assert_eq!(issues[0].source_blade, "cppcheck");
    }

    #[test]
    fn test_normalize_cppcheck_output_falls_back_to_unstructured_text_without_crashing() {
        let repo_path = Path::new("C:/repos/example");
        let payload = "cppcheck: progress 100%\nmain.c:42: warning: suspicious arithmetic";

        let issues = normalize_cppcheck_output(repo_path, repo_path, payload.as_bytes()).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, "warning");
        assert_eq!(issues[0].source_blade, "cppcheck");
        assert!(issues[0]
            .message
            .contains("cppcheck output nao estruturado preservado"));
    }

    #[test]
    fn test_normalize_cppcheck_output_accepts_self_closing_results_as_clean_info() {
        let repo_path = Path::new("C:/repos/example");
        let payload = r#"<?xml version="1.0"?><results/>"#;

        let issues = normalize_cppcheck_output(repo_path, repo_path, payload.as_bytes()).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].level, "info");
        assert_eq!(
            issues[0].message,
            "[INFO] Nenhuma vulnerabilidade encontrada pelo Cppcheck."
        );
        assert_eq!(issues[0].source_blade, "cppcheck");
    }

    #[test]
    fn test_derive_cppcheck_execution_targets_requires_productive_cpp_files() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("src/main.c", "int main() { return 0; }\n");
        executor.write_repo_file("tests/main_test.c", "int main() { return 1; }\n");
        executor.write_repo_file("pkg/service.go", "package service\n");

        let clean_files = vec![
            canonicalize_or_self(executor.repo_path().join("src/main.c")),
            canonicalize_or_self(executor.repo_path().join("tests/main_test.c")),
            canonicalize_or_self(executor.repo_path().join("pkg/service.go")),
        ];

        let targets = derive_cppcheck_execution_targets(executor.repo_path(), &clean_files);

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].scan_targets, vec!["src/main.c".to_string()]);
        assert_eq!(
            targets[0].command_args.as_ref(),
            Some(&cppcheck_args_for_targets(&["src/main.c".to_string()]))
        );
    }
}
