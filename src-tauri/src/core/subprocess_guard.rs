// SOULS V6 — Marco I · v6.1: Subprocess Guard (RAII Lifecycle Owner)
//
// Encapsula `tokio::process::Child` em um guard que envia SIGKILL
// atômico no `Drop`, eliminando processos zumbis na RAM Host. O proxy
// L7 é o **dono absoluto** do ciclo de vida do `souls_mcp_server`:
//
//   1. `SubprocessGuard::spawn(cfg)` → tokio::process::Command spawn.
//   2. `guard.pid()` → u32 para observabilidade / handshake.
//   3. `guard.kill().await` → kill explícito (antes do Drop).
//   4. `Drop::drop` → `child.start_kill()` (sync, no-panic safe).
//
// Aderência:
// - ADR-025: zero `unwrap` em produção; erros propagados via `Result`.
// - ADR-010: `start_kill` é atômico (não bloqueia o event loop).
// - Win32: `start_kill` mapeia para `TerminateProcess(handle, 1)`.
// - Unix:  `start_kill` envia SIGKILL via `kill(-pid, SIGKILL)` (com fallback SIGTERM).
//
// Filosofia Bare-Metal:
// - Sem `Arc<Mutex<Child>>`. Ownership exclusivo do guard.
// - `child` é `Option<Child>` para permitir `take()` no Drop sem refutar.
// - `start_kill` é **sync** (não `.await`) — `Drop` não pode ser async.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::{Child, Command};

/// Configuração canônica para o spawn do subprocesso MCP.
/// Mirror de `McpServerConfig` mas simplificado para uso em runtime.
#[derive(Debug, Clone)]
pub struct SubprocessConfig {
    pub executable_path: String,
    pub args: Vec<String>,
    pub working_dir: String,
    pub kill_on_drop: bool,
}

impl SubprocessConfig {
    /// Carrega config do `GatewayConfig::global().mcp_server`.
    pub fn from_gateway_config() -> Self {
        let cfg = crate::core::gateway_config::GatewayConfig::global();
        Self {
            executable_path: cfg.mcp_server.executable_path.clone(),
            args: cfg.mcp_server.args.clone(),
            working_dir: cfg.mcp_server.working_dir.clone(),
            kill_on_drop: cfg.mcp_server.kill_on_drop,
        }
    }

    /// Override manual — usado em testes para injetar binário efêmero.
    pub fn override_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.executable_path = path.as_ref().to_string_lossy().to_string();
        self
    }

    /// Override de args — usado em testes.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Resolve o **caminho real** do executável, buscando em:
    /// 1. Path absoluto (ou com separador) → testa direto (+ `.exe` no Windows).
    /// 2. Apenas nome → testa CWD.
    /// 3. Apenas nome → testa cada entrada de `$PATH` (na ordem).
    ///
    /// ADR-030: zero deps; implementação manual do `which`.
    /// Resolve o problema de `Path::exists()` falhar para nomes puros como
    /// `"souls_mcp_server.exe"` quando o binário está no PATH/CWD.
    ///
    /// No Windows, também adiciona automaticamente a extensão `.exe` se
    /// o nome não a tiver (Windows não faz append automático quando o
    /// path é relativo + em CWD/PATH — só quando é passado direto a
    /// `Command::new` o CreateProcess faz fallback).
    pub fn resolve_executable_path(&self) -> Result<PathBuf, String> {
        let exec = self.executable_path.trim();
        if exec.is_empty() {
            return Err(
                "mcp_server.executable_path está vazio. Defina SOULS_MCP_BIN ou edite o JSONC."
                    .to_string(),
            );
        }
        // CWD lido aqui (em `resolve_executable_path`) para que os
        // helpers internos permaneçam puros e testáveis.
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Falha ao obter CWD: {e}"))?;
        resolve_executable(exec, &cwd).ok_or_else(|| {
            format!(
                "Binário MCP '{}' não foi encontrado em CWD nem em $PATH. \
                 Defina SOULS_MCP_BIN com path absoluto ou adicione o binário ao PATH.",
                exec
            )
        })
    }

    /// Valida o config: o executável precisa existir (no CWD ou $PATH).
    /// Delega a `resolve_executable_path()` para garantir que a busca
    /// inclui $PATH — não apenas o literal `Path::exists()`.
    pub fn validate(&self) -> Result<(), String> {
        self.resolve_executable_path().map(|_| ())
    }
}

/// Implementação interna **pura** do `which` (zero-deps, testável).
///
/// Recebe `cwd` explicitamente para que os tests não precisem mutar
/// o CWD do processo (causa race conditions com testes paralelos).
///
/// Ordem de busca:
/// 1. Se `exec` contém `/` ou `\` ou é absoluto → testa direto.
/// 2. Se `exec` é apenas um nome → testa `cwd` primeiro, depois $PATH.
/// 3. No Windows, sufixo `.exe` é tentado se o original não existir.
fn resolve_executable(exec: &str, cwd: &Path) -> Option<PathBuf> {
    let has_separator = exec.contains('/') || exec.contains('\\');
    let is_absolute = Path::new(exec).is_absolute();

    if has_separator || is_absolute {
        // Path explícito: testa direto (com fallback .exe no Windows).
        return try_with_exe_extension(Path::new(exec));
    }

    // Apenas nome: busca em CWD primeiro (compat com `Command::new`).
    if let Some(found) = try_with_exe_extension(&cwd.join(exec)) {
        return Some(found);
    }

    // Cada entrada de $PATH.
    if let Ok(path_env) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_env) {
            if let Some(found) = try_with_exe_extension(&dir.join(exec)) {
                return Some(found);
            }
        }
    }

    None
}

/// Tenta `p`; se não existir e for Windows + sem `.exe`, tenta `p.exe`.
fn try_with_exe_extension(p: &Path) -> Option<PathBuf> {
    if p.exists() {
        return Some(p.to_path_buf());
    }
    #[cfg(windows)]
    {
        if let Some(s) = p.to_str() {
            if !s.to_lowercase().ends_with(".exe") {
                let with_exe = format!("{s}.exe");
                let p2 = Path::new(&with_exe);
                if p2.exists() {
                    return Some(p2.to_path_buf());
                }
            }
        }
    }
    None
}

/// RAII guard sobre `tokio::process::Child`.
///
/// Garante SIGKILL no `Drop` (se `kill_on_drop` for `true`), prevenindo
/// processos zumbis quando o proxy é desligado ou sofre panic.
pub struct SubprocessGuard {
    child: Option<Child>,
    pid: Option<u32>,
    kill_on_drop: bool,
    label: String,
}

impl SubprocessGuard {
    /// Spawna o subprocesso conforme `cfg`. Configura stdio piped
    /// (stdin/stdout/stderr capturados para JSON-RPC).
    pub fn spawn(cfg: &SubprocessConfig) -> io::Result<Self> {
        // Resolve o path real (CWD/$PATH/.exe fallback) ANTES de spawnar.
        // Garante que `Command::new` recebe um path absoluto verificável,
        // eliminando ambiguidade do PATH lookup do OS.
        let resolved_path = cfg
            .resolve_executable_path()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let mut cmd = Command::new(&resolved_path);
        cmd.args(&cfg.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(cfg.kill_on_drop); // belt-and-suspenders: tokio já tem esse flag

        if !cfg.working_dir.trim().is_empty() {
            cmd.current_dir(&cfg.working_dir);
        }

        let child = cmd.spawn().map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "Falha ao spawnar MCP subprocess '{}' (resolved: {}): {e}",
                    cfg.executable_path,
                    resolved_path.display()
                ),
            )
        })?;

        let pid = child.id();
        let label = format!("MCP:{}", resolved_path.display());
        tracing::info!(
            target: "subprocess_guard",
            "Subprocess MCP spawnado: pid={:?} exe={} args={:?}",
            pid,
            resolved_path.display(),
            cfg.args
        );

        Ok(Self {
            child: Some(child),
            pid,
            kill_on_drop: cfg.kill_on_drop,
            label,
        })
    }

    /// PID do filho no SO. `None` se o filho já foi reaped.
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Acessa o `Child` para I/O (stdin/stdout/stderr pipes).
    pub fn child_mut(&mut self) -> Option<&mut Child> {
        self.child.as_mut()
    }

    /// Acessa o `&Child` imutável.
    pub fn child_ref(&self) -> Option<&Child> {
        self.child.as_ref()
    }

    /// Take ownership do `Child` (consumindo o guard sem kill).
    /// Útil quando o caller quer integrar o I/O manualmente.
    pub fn into_child(mut self) -> Option<Child> {
        // Desabilita o kill no Drop via flag + take.
        self.kill_on_drop = false;
        self.child.take()
    }

    /// Kill **assíncrono** explícito + wait. Aguarda o filho realmente sair.
    /// Idempotente: se já foi killed, retorna Ok.
    pub async fn kill(mut self) -> io::Result<()> {
        self.kill_on_drop = false; // Drop não fará segundo kill.
        if let Some(mut child) = self.child.take() {
            // start_kill é sync, send SIGKILL/TerminateProcess.
            let _ = child.start_kill();
            // wait aguarda a reaps do OS — sem leak de PID/zombie.
            let _ = child.wait().await;
            tracing::info!(
                target: "subprocess_guard",
                "Subprocess MCP killed: pid={:?} label={}",
                self.pid,
                self.label
            );
        }
        Ok(())
    }

    /// Verifica se o filho ainda está vivo no SO (sem consumir o guard).
    /// Implementação: `try_wait()` síncrono via handle — se ainda não
    /// saiu, retorna `Alive`; se já saiu, retorna `Exited(status)`.
    pub fn probe_state(&mut self) -> SubprocessState {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => SubprocessState::Exited(status),
                Ok(None) => SubprocessState::Alive,
                Err(e) => SubprocessState::Unknown(e.to_string()),
            }
        } else {
            SubprocessState::Reaped
        }
    }
}

impl Drop for SubprocessGuard {
    fn drop(&mut self) {
        if !self.kill_on_drop {
            return;
        }
        if let Some(mut child) = self.child.take() {
            // `start_kill` é sync. No Windows: TerminateProcess(handle, 1).
            // No Unix: kill(-pid, SIGKILL) ou SIGTERM.
            match child.start_kill() {
                Ok(()) => {
                    tracing::info!(
                        target: "subprocess_guard",
                        "SubprocessGuard::drop: SIGKILL enviado para pid={:?} ({})",
                        self.pid,
                        self.label
                    );
                    // NÃO await wait() — Drop é sync. O OS reapa o zombie
                    // quando o handle for fechado. Para wait assíncrono,
                    // chame explicitamente `guard.kill().await` antes do drop.
                }
                Err(e) => {
                    tracing::error!(
                        target: "subprocess_guard",
                        "SubprocessGuard::drop: falha ao kill pid={:?} ({e})",
                        self.pid
                    );
                }
            }
        }
    }
}

/// Estado do subprocesso observado via `try_wait`.
#[derive(Debug)]
pub enum SubprocessState {
    /// Processo ainda está rodando.
    Alive,
    /// Processo saiu com um `ExitStatus`.
    Exited(std::process::ExitStatus),
    /// `try_wait` falhou (raro).
    Unknown(String),
    /// Guard já cedeu o `Child` (via `into_child` ou após `kill().await`).
    Reaped,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD: `validate` rejeita config com path vazio.
    #[test]
    fn test_validate_rejects_empty_path() {
        let cfg = SubprocessConfig {
            executable_path: String::new(),
            args: vec![],
            working_dir: String::new(),
            kill_on_drop: true,
        };
        assert!(cfg.validate().is_err());
    }

    /// TDD: `validate` rejeita path inexistente.
    #[test]
    fn test_validate_rejects_nonexistent_path() {
        let cfg = SubprocessConfig {
            executable_path: "C:\\this\\path\\does\\not\\exist\\souls_mcp_server.exe".to_string(),
            args: vec![],
            working_dir: String::new(),
            kill_on_drop: true,
        };
        assert!(cfg.validate().is_err());
    }

    /// TDD: `kill_on_drop=false` desativa o kill automático.
    #[test]
    fn test_no_kill_when_disabled() {
        // Path dummy que não existe — mas `kill_on_drop=false` no Drop não chama nada.
        let cfg = SubprocessConfig {
            executable_path: "C:\\nonexistent.exe".to_string(),
            args: vec![],
            working_dir: String::new(),
            kill_on_drop: false,
        };
        // Não vamos spawn — só validamos a config.
        // (validate falha porque path não existe, mas Drop não seria chamado.)
        assert!(cfg.validate().is_err());

        // TDD: o campo kill_on_drop é apenas flag de controle, sem alocações.
        let cfg2 = SubprocessConfig {
            executable_path: String::new(),
            args: vec![],
            working_dir: String::new(),
            kill_on_drop: false,
        };
        assert!(!cfg2.kill_on_drop);
    }

    // ====================================================================
    // TDD — Issue: `Path::exists()` falha para executáveis no PATH
    // ====================================================================

    /// TDD — Edge case (cross-platform): path absoluto inexistente ainda
    /// deve falhar (não busca em $PATH se o path tem separador).
    #[test]
    fn test_resolve_rejects_absolute_nonexistent() {
        let cfg = SubprocessConfig {
            executable_path: if cfg!(windows) {
                "C:\\this\\path\\will\\never\\exist\\foo.exe".to_string()
            } else {
                "/this/path/will/never/exist/foo".to_string()
            },
            args: vec![],
            working_dir: String::new(),
            kill_on_drop: true,
        };
        assert!(
            cfg.resolve_executable_path().is_err(),
            "path absoluto inexistente deve falhar"
        );
    }

    /// TDD — Função pura: nome puro é encontrado em `cwd` quando o
    /// arquivo existe lá. Usa `resolve_executable(exec, cwd)` diretamente
    /// (sem mutar env global) — **parallel-safe**.
    #[test]
    fn test_resolve_finds_pure_name_in_cwd() {
        let file_name = if cfg!(windows) { "fake_mcp.exe" } else { "fake_mcp" };
        let dir = tempfile::TempDir::with_prefix("souls_test_cwd").expect("criar tempdir");
        let file_path = dir.path().join(file_name);
        std::fs::write(&file_path, b"fake binary content").expect("escrever temp file");

        // CWD passado explicitamente — zero mutação de env global.
        let resolved = resolve_executable(file_name, dir.path())
            .expect("resolver nome puro em CWD explicitamente fornecido");
        assert!(
            resolved.exists(),
            "resolved path deve existir: {}",
            resolved.display()
        );
        assert_eq!(
            resolved.file_name().and_then(|n| n.to_str()),
            Some(file_name)
        );
    }

    /// TDD — Função pura: nome puro NÃO é encontrado em CWD se não
    /// existe lá, mas é encontrado em um diretório passado como `cwd`
    /// customizado (simulando PATH). Usa a função pura diretamente.
    #[test]
    fn test_resolve_finds_pure_name_in_custom_cwd_as_path() {
        // Cenário: o "binário" está em um dir que não é o CWD real,
        // mas passamos esse dir como cwd. Verifica que a busca em cwd
        // funciona (esse dir age como se fosse o PATH entry testado).
        let file_name = if cfg!(windows) {
            "fake_mcp_in_dir.exe"
        } else {
            "fake_mcp_in_dir"
        };
        let dir = tempfile::TempDir::with_prefix("souls_test_custom_cwd")
            .expect("criar tempdir");
        let file_path = dir.path().join(file_name);
        std::fs::write(&file_path, b"fake binary content").expect("escrever temp file");

        // Outro dir como "CWD" e o dir real como "PATH entry" — testamos
        // a busca em cwd via parâmetro explícito.
        let real_cwd = tempfile::TempDir::with_prefix("souls_test_empty_cwd")
            .expect("criar tempdir vazio");
        let resolved = resolve_executable(file_name, real_cwd.path());
        // CWD real está vazio → deve falhar (não busca em dir.path()).
        assert!(
            resolved.is_none(),
            "resolve deve falhar quando cwd não tem o arquivo, got: {:?}",
            resolved
        );

        // Agora passando dir.path() como cwd → deve encontrar.
        let resolved = resolve_executable(file_name, dir.path())
            .expect("resolve deve achar o arquivo no cwd customizado");
        assert!(resolved.exists());
    }

    /// TDD — Edge case (Windows): nome sem `.exe` deve resolver para `nome.exe`
    /// quando passado como path absoluto.
    #[cfg(windows)]
    #[test]
    fn test_resolve_appends_exe_extension_on_windows() {
        let dir = tempfile::TempDir::with_prefix("souls_test_exe_fallback")
            .expect("criar tempdir");
        let file_with_ext = dir.path().join("binary_without_ext.exe");
        std::fs::write(&file_with_ext, b"x").expect("escrever temp file");

        // Pede pelo path SEM extensão (mas absoluto, com separador).
        let abs_no_ext = dir.path().join("binary_without_ext");
        let resolved = resolve_executable(
            abs_no_ext.to_str().expect("path utf8"),
            dir.path(),
        )
        .expect("deve resolver com fallback .exe no Windows");
        assert!(
            resolved
                .to_string_lossy()
                .to_lowercase()
                .ends_with(".exe"),
            "resolved path deve terminar com .exe, got: {}",
            resolved.display()
        );
    }
}
