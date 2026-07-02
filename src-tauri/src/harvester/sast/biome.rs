use super::JsLintProfile;

pub(crate) fn biome_args(scan_targets: &[String]) -> Vec<String> {
    biome_args_for_profile(scan_targets, JsLintProfile::Health)
}

pub(crate) fn biome_args_for_profile(scan_targets: &[String], profile: JsLintProfile) -> Vec<String> {
    let mut args = vec![
        "lint".to_string(),
        "--reporter=json".to_string(),
        "--no-errors-on-unmatched".to_string(),
        "--skip-parse-errors".to_string(),
        "--vcs-enabled=true".to_string(),
        "--vcs-client-kind=git".to_string(),
        "--vcs-use-ignore-file=true".to_string(),
        "--files-ignore-unknown=true".to_string(),
    ];
    match profile {
        JsLintProfile::UnsafeHotspot => {
            args.push("--only=lint/security".to_string());
        }
        JsLintProfile::Health => {
            args.push("--only=lint/complexity".to_string());
        }
    }
    if scan_targets.is_empty() {
        args.push(".".to_string());
    } else {
        args.extend(scan_targets.iter().cloned());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::router::StaticAnalysisBlade;
    use crate::harvester::sast::blade_command;

    #[test]
    fn test_biome_args_accept_explicit_scan_targets_with_health_profile() {
        let scan_targets = vec!["src/index.ts".to_string(), "src/server.ts".to_string()];
        let (biome_binary, biome_args) =
            blade_command(StaticAnalysisBlade::Biome, &scan_targets, None);

        assert_eq!(biome_binary, "biome");
        assert_eq!(biome_args.first().map(String::as_str), Some("lint"));
        assert!(!biome_args.iter().any(|arg| arg == "check"));
        assert!(biome_args.iter().any(|arg| arg == "--no-errors-on-unmatched"));
        assert!(biome_args.iter().any(|arg| arg == "--only=lint/complexity"));
        assert!(!biome_args.iter().any(|arg| arg == "--only=lint/security"));
        assert!(!biome_args.iter().any(|arg| arg == "--only=lint/suspicious"));
        assert!(biome_args.ends_with(&scan_targets));
    }

    #[test]
    fn test_biome_args_use_security_only_for_blob06_profile() {
        let scan_targets = vec!["src/index.ts".to_string()];
        let biome_args = biome_args_for_profile(&scan_targets, JsLintProfile::UnsafeHotspot);

        assert!(biome_args.iter().any(|arg| arg == "--only=lint/security"));
        assert!(!biome_args.iter().any(|arg| arg == "--only=lint/complexity"));
        assert!(!biome_args.iter().any(|arg| arg == "--only=lint/suspicious"));
        assert!(biome_args.ends_with(&scan_targets));
    }
}
