use std::path::PathBuf;
use anthropophagy::harvester::sandbox::{SandboxOrchestrator, SandboxPolicy};
use anthropophagy::harvester::git::RepoPath;
use souls_mc_lib::telemetry::{enable_virtual_terminal, init_cli_tracing, parse_log_level_from_env};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializa o tracing
    enable_virtual_terminal();
    let level = parse_log_level_from_env();
    init_cli_tracing(level);

    println!("====================================================");
    println!("   🦅 SOULS GAIOLA DE SILÍCIO: TESTADOR DO BIOME 🦅   ");
    println!("====================================================");

    // 2. Define o caminho do repositório/workspace
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().ok_or("Erro ao resolver raiz do projeto")?;
    println!("Repositório ativo: {}", repo_root.display());

    // 3. Instancia a Gaiola de Silício
    println!("\n[+] Instanciando SandboxOrchestrator...");
    let repo_path = RepoPath(repo_root.to_path_buf());
    let sandbox = SandboxOrchestrator::create(&repo_path, SandboxPolicy::ReadWrite).await?;
    println!("[OK] Sandbox instanciado com sucesso!");

    // 4. Executa o oxlint na Gaiola
    println!("\n[+] Disparando execução do Oxlint...");
    let command = "oxlint";
    let args = vec!["--version"];
    
    let result = sandbox.execute_in_appcontainer_in_dir(command, &args, 30, repo_root).await;

    match result {
        Ok(stdout) => {
            println!("\n[SUCESSO] Oxlint executado com Exit Code 0!");
            println!("Stdout retornado:\n{}", String::from_utf8_lossy(&stdout));
        }
        Err(e) => {
            println!("\n[FALHA] Execução falhou: {:?}", e);
            if let anthropophagy::harvester::sandbox::SandboxError::ProcessNonZeroExit { exit_code, stderr, stdout } = e {
                println!("Exit Code: {}", exit_code);
                println!("Stdout:\n{}", String::from_utf8_lossy(&stdout));
                println!("Stderr:\n{}", stderr);
            }
        }
    }

    println!("====================================================");
    Ok(())
}
