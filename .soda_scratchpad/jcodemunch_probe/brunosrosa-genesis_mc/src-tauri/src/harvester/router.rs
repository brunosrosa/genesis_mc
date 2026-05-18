use super::detect::StackProfile;
use super::git::RepoPath;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExtractionTask {
    RunJCodemunch,
    RunOxc,
    /// Descoberta estatica de testes por AST/estrutura, sem executar runners nativos.
    DiscoverTests,
    ExtractManifests,
    RunStaticAnalysis,
    FetchCommunityMeta,
    ExtractOpsBlueprint,
}

impl ExtractionTask {
    /// Lei da Compressao Topologica: sidecars e extratores estruturais devem emitir blocos
    /// hierarquicos por arquivo, com poda de granularidade interna, ao inves de saida plana.
    pub fn enforces_topology_compression(&self) -> bool {
        matches!(
            self,
            Self::RunJCodemunch | Self::RunOxc | Self::DiscoverTests
        )
    }
}

pub struct ExtractionInput<'a> {
    pub profile: StackProfile,
    pub repo_path: &'a RepoPath,
}

use crate::harvester::detect::SingleStack;

pub struct ExtractionRouter;

fn single_stack_tasks(stack: &SingleStack) -> Vec<ExtractionTask> {
    match stack {
        SingleStack::Rust => vec![
            ExtractionTask::RunJCodemunch,
            ExtractionTask::DiscoverTests,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::NodeJS => vec![
            ExtractionTask::RunJCodemunch,
            ExtractionTask::RunOxc,
            ExtractionTask::DiscoverTests,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::Go => vec![
            ExtractionTask::RunJCodemunch,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::Python => vec![
            ExtractionTask::RunJCodemunch,
            ExtractionTask::DiscoverTests,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::JVM => vec![
            ExtractionTask::RunJCodemunch,
            ExtractionTask::ExtractManifests,
            ExtractionTask::RunStaticAnalysis,
            ExtractionTask::FetchCommunityMeta,
            ExtractionTask::ExtractOpsBlueprint,
        ],
        SingleStack::DotNet => vec![
            ExtractionTask::RunJCodemunch,
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
        ExtractionTask::ExtractManifests,
        ExtractionTask::FetchCommunityMeta,
        ExtractionTask::ExtractOpsBlueprint,
    ]
}

/// Converte um `StackProfile` em `SingleStack` para delegar ao mapeamento canônico.
/// Retorna `None` para `Unknown` e `Mixed` (tratados separadamente).
fn profile_to_single(profile: &StackProfile) -> Option<SingleStack> {
    match profile {
        StackProfile::Rust => Some(SingleStack::Rust),
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
        if let Some(single) = profile_to_single(&input.profile) {
            return single_stack_tasks(&single);
        }

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
            // Inalcançável: profile_to_single já cobriu todas as variantes individuais.
            // O fallback garante PT-ROUTE-3 (vetor nunca vazio).
            _ => unknown_fallback(),
        }
    }
}

pub fn route(input: ExtractionInput<'_>) -> Vec<ExtractionTask> {
    ExtractionRouter::route(input)
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
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunJCodemunch,
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
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunJCodemunch,
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
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunJCodemunch,
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
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunJCodemunch,
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
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunJCodemunch,
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
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunJCodemunch,
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
        };
        let tasks = route(input);
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::ExtractManifests,
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
        };
        let tasks = route(input);
        // NodeJS traz RunOxc, Rust traz as outras. Elas devem ser unidas e deduplicadas sem quebrar a ordem.
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunJCodemunch,
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
        };
        let tasks = route(input);
        // Python traz DiscoverTests; Go nao traz RunOxc. O vetor final deve manter a ordem e deduplicar.
        assert_eq!(
            tasks,
            vec![
                ExtractionTask::RunJCodemunch,
                ExtractionTask::ExtractManifests,
                ExtractionTask::RunStaticAnalysis,
                ExtractionTask::FetchCommunityMeta,
                ExtractionTask::ExtractOpsBlueprint,
                ExtractionTask::DiscoverTests,
            ]
        );
    }

    #[test]
    fn test_route_never_empty() {
        let repo = mock_repo_path();
        
        // Testa todos os perfis possíveis, garantindo que nenhum retorna vetor vazio
        let profiles = vec![
            StackProfile::Rust,
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
            };
            let tasks = route(input);
            assert!(!tasks.is_empty(), "Retorno vazio para o perfil!");
        }
    }
}
