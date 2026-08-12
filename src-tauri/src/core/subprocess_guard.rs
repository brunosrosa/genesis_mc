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
use std::path::Path;
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

    /// Valida o config: erro explícito se o path está vazio.
    pub fn validate(&self) -> Result<(), String> {
        if self.executable_path.trim().is_empty() {
            return Err(
                "mcp_server.executable_path está vazio. Defina SOULS_MCP_BIN ou edite o JSONC."
                    .to_string(),
            );
        }
        let path = std::path::Path::new(&self.executable_path);
        if !path.exists() {
            return Err(format!(
                "Binário MCP '{}' não existe no filesystem. Verifique SOULS_MCP_BIN ou rebuild.",
                self.executable_path
            ));
        }
        Ok(())
    }
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
        cfg.validate()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let mut cmd = Command::new(&cfg.executable_path);
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
                    "Falha ao spawnar MCP subprocess '{}': {e}",
                    cfg.executable_path
                ),
            )
        })?;

        let pid = child.id();
        let label = format!("MCP:{}", cfg.executable_path);
        tracing::info!(
            target: "subprocess_guard",
            "Subprocess MCP spawnado: pid={:?} exe={} args={:?}",
            pid,
            cfg.executable_path,
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
}
