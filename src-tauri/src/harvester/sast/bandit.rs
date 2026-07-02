pub fn bandit_args(scan_targets: &[String]) -> Vec<String> {
    let mut args = vec![
        "-f".to_string(),
        "json".to_string(),
        "-q".to_string(),
        "-s".to_string(),
        "B101".to_string(),
    ];
    if scan_targets.is_empty() {
        args.push("-r".to_string());
        args.push(".".to_string());
    } else {
        args.extend(scan_targets.iter().cloned());
    }
    args
}

#[cfg(test)]
mod tests {
    use crate::harvester::router::StaticAnalysisBlade;
    use crate::harvester::sast::{blade_command, SandboxExecutor};
    use crate::harvester::sast::test_utils::MockExecutor;

    #[test]
    fn test_normalize_bandit_output_drops_assert_noise_even_if_tool_leaks_it() {
        let executor = MockExecutor::new(Vec::new());
        executor.write_repo_file("src/app.py", "def run():\n    return 1\n");
        let payload = br#"{
            "results": [
                {
                    "filename": "tests/test_app.py",
                    "issue_severity": "LOW",
                    "issue_text": "Use of assert detected.",
                    "line_number": 7
                },
                {
                    "filename": "src/app.py",
                    "issue_severity": "HIGH",
                    "issue_text": "Potential shell injection via subprocess",
                    "line_number": 12
                }
            ]
        }"#;

        let issues = crate::harvester::sast::normalize_sast_output(
            executor.repo_path(),
            executor.repo_path(),
            StaticAnalysisBlade::Bandit,
            payload,
        )
        .unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].file, "src/app.py");
        assert!(issues[0].message.contains("Potential shell injection"));
    }

    #[test]
    fn test_python_linter_args_accept_explicit_scan_targets_and_skip_bandit_b101() {
        let scan_targets = vec!["src/app.py".to_string(), "services/api.py".to_string()];
        let (ruff_binary, ruff_args) = blade_command(StaticAnalysisBlade::Ruff, &scan_targets, None);
        let (bandit_binary, bandit_args) =
            blade_command(StaticAnalysisBlade::Bandit, &scan_targets, None);

        assert_eq!(ruff_binary, "ruff");
        assert_eq!(bandit_binary, "bandit");
        assert_eq!(ruff_args[..3], ["check", "--output-format", "json"]);
        assert!(ruff_args.windows(2).any(|pair| pair == ["--ignore", "D,F401,UP,W"]));
        assert!(ruff_args.ends_with(&scan_targets));
        assert!(bandit_args.windows(2).any(|pair| pair == ["-s", "B101"]));
        assert!(bandit_args.ends_with(&scan_targets));
        assert!(!bandit_args.iter().any(|arg| arg == "-r"));
    }
}
