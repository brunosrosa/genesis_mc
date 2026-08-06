use std::collections::BTreeSet;

use super::detect::StackProfile;
use super::git::RepoPath;

pub const PHASE0_BLOB_TYPES: [&str; 11] = [
    "blob_01_promessa_readme",
    "blob_02_dependency_manifest",
    "blob_03_test_intent",
    "blob_04_repo_outline",
    "blob_05_architecture_map",
    "blob_06_unsafe_hotspots",
    "blob_07_ops_blueprint",
    "blob_08_health_report",
    "blob_09_community_meta",
    "blob_10_souls_canon_context",
    "blob_11_ux_contracts",
];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExtractionTask {
    RunNativeAstParser,
    RunOxc,
    /// Descoberta estatica de testes por AST/estrutura, sem executar runners nativos.
    DiscoverTests,
    ExtractManifests,
    RunStaticAnalysis,
    FetchCommunityMeta,
    ExtractOpsBlueprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticAnalysisBlade {
    RustClippy,
    Cppcheck,
    Sobelow,
    Biome,
    Oxc,
    Ruff,
    Bandit,
    Govulncheck,
    Opengrep,
}

impl ExtractionTask {
    /// Lei da Compressao Topologica: sidecars e extratores estruturais devem emitir blocos
    /// hierarquicos por arquivo, com poda de granularidade interna, ao inves de saida plana.
    pub fn enforces_topology_compression(&self) -> bool {
        matches!(
            self,
            Self::RunNativeAstParser | Self::RunOxc | Self::DiscoverTests
        )
    }
}

pub struct ExtractionInput<'a> {
    pub profile: StackProfile,
    pub repo_path: &'a RepoPath,
    pub requested_blobs: Option<&'a BlobSelection>,
}

use crate::harvester::detect::SingleStack;

pub struct ExtractionRouter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobSelection {
    artifact_types: BTreeSet<&'static str>,
}

impl BlobSelection {
    pub fn all() -> Self {
        Self {
            artifact_types: PHASE0_BLOB_TYPES.into_iter().collect(),
        }
    }

    pub fn from_csv(raw: &str) -> Result<Self, String> {
        let mut artifact_types = BTreeSet::new();
        for item in raw.split(',') {
            let spec = item.trim();
            if spec.is_empty() {
                continue;
            }
            let artifact_type = resolve_blob_spec(spec).ok_or_else(|| {
                format!(
                    "Valor inválido em --only-blobs: '{spec}'. Use códigos como 06,08 ou nomes blob_XX_*."
                )
            })?;
            artifact_types.insert(artifact_type);
        }

        if artifact_types.is_empty() {
            return Err(
                "A flag --only-blobs exige ao menos um código válido, por exemplo: --only-blobs 06,08"
                    .to_string(),
            );
        }

        Ok(Self { artifact_types })
    }

    pub fn contains_artifact(&self, artifact_type: &str) -> bool {
        self.artifact_types.contains(artifact_type)
    }

    pub fn expected_artifact_types(&self) -> Vec<String> {
        PHASE0_BLOB_TYPES
            .iter()
            .filter(|artifact_type| self.contains_artifact(artifact_type))
            .map(|artifact_type| (*artifact_type).to_string())
            .collect()
    }

    pub fn allows_task(&self, task: &ExtractionTask) -> bool {
        match task {
            ExtractionTask::RunNativeAstParser => {
                self.contains_artifact("blob_04_repo_outline")
                    || self.contains_artifact("blob_05_architecture_map")
            }
            ExtractionTask::RunOxc => self.contains_artifact("blob_11_ux_contracts"),
            ExtractionTask::DiscoverTests => self.contains_artifact("blob_03_test_intent"),
            ExtractionTask::ExtractManifests => self.contains_artifact("blob_02_dependency_manifest"),
            ExtractionTask::RunStaticAnalysis => {
                self.contains_artifact("blob_06_unsafe_hotspots")
                    || self.contains_artifact("blob_08_health_report")
            }
            ExtractionTask::FetchCommunityMeta => self.contains_artifact("blob_09_community_meta"),
            ExtractionTask::ExtractOpsBlueprint => self.contains_artifact("blob_07_ops_blueprint"),
        }
    }
}

fn resolve_blob_spec(spec: &str) -> Option<&'static str> {
    match spec.trim().to_ascii_lowercase().as_str() {
        "01" | "1" | "blob_01_promessa_readme" => Some("blob_01_promessa_readme"),
        "02" | "2" | "blob_02_dependency_manifest" => Some("blob_02_dependency_manifest"),
        "03" | "3" | "blob_03_test_intent" => Some("blob_03_test_intent"),
        "04" | "4" | "blob_04_repo_outline" => Some("blob_04_repo_outline"),
        "05" | "5" | "blob_05_architecture_map" => Some("blob_05_architecture_map"),
        "06" | "6" | "blob_06_unsafe_hotspots" => Some("blob_06_unsafe_hotspots"),
        "07" | "7" | "blob_07_ops_blueprint" => Some("blob_07_ops_blueprint"),
        "08" | "8" | "blob_08_health_report" => Some("blob_08_health_report"),
        "09" | "9" | "blob_09_community_meta" => Some("blob_09_community_meta"),
        "10" | "blob_10_souls_canon_context" => Some("blob_10_souls_canon_context"),
        "11" | "blob_11_ux_contracts" => Some("blob_11_ux_contracts"),
        _ => None,
    }
}

fn single_stack_tasks(stack: &SingleStack) -> Vec<ExtractionTask> {
    match stack {
        SingleStack::Rust => vec![
            ExtractionTask::RunNativeAstParser,
            ExtractionTask::DiscoverTests,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::CCpp => vec![
            ExtractionTask::RunNativeAstParser,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::Elixir => vec![
            ExtractionTask::RunNativeAstParser,
            ExtractionTask::DiscoverTests,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::NodeJS => vec![
            ExtractionTask::RunNativeAstParser,
            ExtractionTask::RunOxc,
            ExtractionTask::DiscoverTests,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::Go => vec![
            ExtractionTask::RunNativeAstParser,
            ExtractionTask::DiscoverTests,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::Python => vec![
            ExtractionTask::RunNativeAstParser,
            ExtractionTask::DiscoverTests,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::JVM => vec![
            ExtractionTask::RunNativeAstParser,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::DotNet => vec![
            ExtractionTask::RunNativeAstParser,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
    }
}

/// Fallback mínimo de 3 tarefas genéricas para perfis sem stack conhecida.
fn unknown_fallback() -> Vec<ExtractionTask> {
    vec![
        ExtractionTask::RunNativeAstParser,
        ExtractionTask::ExtractManifests,
        ExtractionTask::RunStaticAnalysis,
        ExtractionTask::FetchCommunityMeta,
        ExtractionTask::ExtractOpsBlueprint,
    ]
}

fn static_analysis_blades_for_stack(stack: &SingleStack) -> Vec<StaticAnalysisBlade> {
    match stack {
        SingleStack::Rust => vec![StaticAnalysisBlade::RustClippy],
        SingleStack::CCpp => vec![StaticAnalysisBlade::Cppcheck],
        SingleStack::Elixir => vec![StaticAnalysisBlade::Sobelow],
        SingleStack::NodeJS => vec![StaticAnalysisBlade::Biome],
        SingleStack::Python => vec![StaticAnalysisBlade::Ruff, StaticAnalysisBlade::Bandit],
        SingleStack::Go => vec![StaticAnalysisBlade::Govulncheck],
        SingleStack::JVM | SingleStack::DotNet => vec![StaticAnalysisBlade::Opengrep],
    }
}

fn append_global_fallback(blades: &mut Vec<StaticAnalysisBlade>) {
    if !blades.contains(&StaticAnalysisBlade::Opengrep) {
        blades.push(StaticAnalysisBlade::Opengrep);
    }
}

/// Converte um `StackProfile` em `SingleStack` para delegar ao mapeamento canônico.
/// Retorna `None` para `Unknown` e `Mixed` (tratados separadamente).
fn profile_to_single(profile: &StackProfile) -> Option<SingleStack> {
    match profile {
        StackProfile::Rust => Some(SingleStack::Rust),
        StackProfile::CCpp => Some(SingleStack::CCpp),
        StackProfile::Elixir => Some(SingleStack::Elixir),
        StackProfile::NodeJS => Some(SingleStack::NodeJS),
        StackProfile::Go => Some(SingleStack::Go),
        StackProfile::Python => Some(SingleStack::Python),
        StackProfile::JVM => Some(SingleStack::JVM),
        StackProfile::DotNet => Some(SingleStack::DotNet),
        StackProfile::Unknown | StackProfile::Mixed(_) => None,
    }
}

impl ExtractionRouter {
    /// Roteia as tarefas de extração com base no perfil de stack detectado.
    /// Esta função é pura, determinística e síncrona.
    pub fn route(input: ExtractionInput<'_>) -> Vec<ExtractionTask> {
        // Tenta converter para SingleStack primeiro — cobre Rust/NodeJS/Go/Python/JVM/DotNet
        let tasks = if let Some(single) = profile_to_single(&input.profile) {
            single_stack_tasks(&single)
        } else {
            match input.profile {
            StackProfile::Unknown => unknown_fallback(),
            StackProfile::Mixed(stacks) => {
                let mut tasks = Vec::with_capacity(7);
                for s in &stacks {
                    for t in single_stack_tasks(s) {
                        if !tasks.contains(&t) {
                            tasks.push(t);
                        }
                    }
                }
                if tasks.is_empty() {
                    tasks = unknown_fallback();
                }
                tasks
            }
            _ => unknown_fallback(),
            }
        };

        if let Some(selection) = input.requested_blobs {
            tasks
                .into_iter()
                .filter(|task| selection.allows_task(task))
                .collect()
        } else {
            tasks
        }
    }
}

pub fn route(input: ExtractionInput<'_>) -> Vec<ExtractionTask> {
    ExtractionRouter::route(input)
}

pub fn route_static_analysis_blades(profile: &StackProfile) -> Vec<StaticAnalysisBlade> {
    if let Some(single) = profile_to_single(profile) {
        let mut blades = static_analysis_blades_for_stack(&single);
        append_global_fallback(&mut blades);
        return blades;
    }

    match profile {
        StackProfile::Unknown => vec![StaticAnalysisBlade::Opengrep],
        StackProfile::Mixed(stacks) => {
            let mut blades = Vec::new();
            for stack in stacks {
                for blade in static_analysis_blades_for_stack(stack) {
                    if !blades.contains(&blade) {
                        blades.push(blade);
                    }
                }
            }
            if blades.is_empty() {
                vec![StaticAnalysisBlade::Opengrep]
            } else {
                append_global_fallback(&mut blades);
                blades
            }
        }
        _ => vec![StaticAnalysisBlade::Opengrep],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harvester::detect::SingleStack;
    use std::path::PathBuf;

    // Helper para criar um RepoPath mockado de forma segura sem tocar no disco nos testes puros
    fn mock_repo_path() -> RepoPath {
        RepoPath(PathBuf::from("mock_ramdisk_path"))
    }

    #[test]
    fn test_route_rust() {
        let repo = mock_repo_path();
        let input = ExtractionInput {
            profile: StackProfile::Rust,
            repo_path: &repo,
            requested_blobs: None,
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunNativeAstParser,
                ExtractionTask::DiscoverTests,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
            ]
        );
    }

    #[test]
    fn test_route_nodejs() {
        let repo = mock_repo_path();
        let input = ExtractionInput {
            profile: StackProfile::NodeJS,
            repo_path: &repo,
            requested_blobs: None,
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunNativeAstParser,
                ExtractionTask::RunOxc,
                ExtractionTask::DiscoverTests,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
            ]
        );
    }

    #[test]
    fn test_route_go() {
        let repo = mock_repo_path();
        let input = ExtractionInput {
            profile: StackProfile::Go,
            repo_path: &repo,
            requested_blobs: None,
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunNativeAstParser,
                ExtractionTask::DiscoverTests,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
            ]
        );
    }

    #[test]
    fn test_route_python() {
        let repo = mock_repo_path();
        let input = ExtractionInput {
            profile: StackProfile::Python,
            repo_path: &repo,
            requested_blobs: None,
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunNativeAstParser,
                ExtractionTask::DiscoverTests,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
            ]
        );
    }

    #[test]
    fn test_route_jvm() {
        let repo = mock_repo_path();
        let input = ExtractionInput {
            profile: StackProfile::JVM,
            repo_path: &repo,
            requested_blobs: None,
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunNativeAstParser,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
            ]
        );
    }

    #[test]
    fn test_route_dotnet() {
        let repo = mock_repo_path();
        let input = ExtractionInput {
            profile: StackProfile::DotNet,
            repo_path: &repo,
            requested_blobs: None,
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunNativeAstParser,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
            ]
        );
    }

    #[test]
    fn test_route_unknown_fallback() {
        let repo = mock_repo_path();
        let input = ExtractionInput {
            profile: StackProfile::Unknown,
            repo_path: &repo,
            requested_blobs: None,
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunNativeAstParser,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
            ]
        );
    }

    #[test]
    fn test_route_mixed_dedup() {
        let repo = mock_repo_path();
        // Mixed contendo Rust e NodeJS
        let input = ExtractionInput {
            profile: StackProfile::Mixed(vec![SingleStack::Rust, SingleStack::NodeJS]),
            repo_path: &repo,
            requested_blobs: None,
        };
        let tasks = route(input);
        // NodeJS traz RunOxc, Rust traz as outras. Elas devem ser unidas e deduplicadas sem quebrar a ordem.
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunNativeAstParser,
                ExtractionTask::DiscoverTests,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
                ExtractionTask::RunOxc,
            ]
        );
    }

    #[test]
    fn test_route_mixed_no_frontend() {
        let repo = mock_repo_path();
        // Mixed contendo Go e Python
        let input = ExtractionInput {
            profile: StackProfile::Mixed(vec![SingleStack::Go, SingleStack::Python]),
            repo_path: &repo,
            requested_blobs: None,
        };
        let tasks = route(input);
        // Python traz DiscoverTests; Go nao traz RunOxc. O vetor final deve manter a ordem e deduplicar.
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunNativeAstParser,
                ExtractionTask::DiscoverTests,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
            ]
        );
    }

    #[test]
    fn test_route_never_empty() {
        let repo = mock_repo_path();
        
        // Testa todos os perfis possíveis, garantindo que nenhum retorna vetor vazio
        let profiles = vec![
            StackProfile::Rust,
            StackProfile::CCpp,
            StackProfile::Elixir,
            StackProfile::NodeJS,
            StackProfile::Go,
            StackProfile::Python,
            StackProfile::JVM,
            StackProfile::DotNet,
            StackProfile::Unknown,
            StackProfile::Mixed(vec![]),
            StackProfile::Mixed(vec![SingleStack::Rust]),
        ];

        for p in profiles {
            let input = ExtractionInput {
                profile: p,
                repo_path: &repo,
                requested_blobs: None,
            };
            let tasks = route(input);
            assert!(!tasks.is_empty(), "Retorno vazio para o perfil!");
        }
    }

    #[test]
    fn test_route_only_blobs_06_08_keeps_only_static_analysis() {
        let repo = mock_repo_path();
        let requested_blobs = BlobSelection::from_csv("06,08").unwrap();
        let input = ExtractionInput {
            profile: StackProfile::Mixed(vec![SingleStack::Rust, SingleStack::NodeJS]),
            repo_path: &repo,
            requested_blobs: Some(&requested_blobs),
        };

        assert_eq!(route(input), vec![ExtractionTask::RunStaticAnalysis]);
    }

    #[test]
    fn test_blob_selection_parses_codes_and_names() {
        let selection = BlobSelection::from_csv("06,blob_08_health_report,11").unwrap();
        assert_eq!(
            selection.expected_artifact_types(),
            vec![
                "blob_06_unsafe_hotspots".to_string(),
                "blob_08_health_report".to_string(),
                "blob_11_ux_contracts".to_string(),
            ]
        );
    }

    #[test]
    fn test_static_analysis_blades_route_mixed_rust_and_cpp() {
        let blades = route_static_analysis_blades(&StackProfile::Mixed(vec![
            SingleStack::Rust,
            SingleStack::CCpp,
        ]));

        assert_eq!(
            blades,
            vec![
                StaticAnalysisBlade::RustClippy,
                StaticAnalysisBlade::Cppcheck,
                StaticAnalysisBlade::Opengrep,
            ]
        );
    }

    #[test]
    fn test_static_analysis_blades_route_nodejs_family_to_biome_and_opengrep() {
        let blades = route_static_analysis_blades(&StackProfile::NodeJS);

        assert_eq!(blades, vec![StaticAnalysisBlade::Biome, StaticAnalysisBlade::Opengrep]);
    }

    #[test]
    fn test_static_analysis_blades_route_unknown_to_opengrep() {
        let blades = route_static_analysis_blades(&StackProfile::Unknown);
        assert_eq!(blades, vec![StaticAnalysisBlade::Opengrep]);
    }

    #[test]
    fn test_static_analysis_blades_route_go_adds_opengrep_fallback() {
        let blades = route_static_analysis_blades(&StackProfile::Go);
        assert_eq!(
            blades,
            vec![StaticAnalysisBlade::Govulncheck, StaticAnalysisBlade::Opengrep]
        );
    }
}
