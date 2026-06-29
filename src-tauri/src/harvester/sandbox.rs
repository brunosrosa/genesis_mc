use std::env;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use sysinfo::{Pid, System};
use thiserror::Error;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};
use super::git::RepoPath;
#[cfg(target_os = "windows")]
use std::mem::size_of;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
    JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxPolicy {
    ReadOnly,
    ReadWrite,
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum SandboxError {
    #[error("Privilege error: {reason}")]
    PrivilegeError { reason: String },

    #[error("Policy violation: {detail}")]
    PolicyViolation { detail: String },

    #[error("Spawn failed: {reason}")]
    ProcessSpawnFailed { reason: String },

    /// Processo terminou com exit code != 0. Diferente de ProcessSpawnFailed,
    /// aqui o processo NASCEU e EXECUTOU, mas retornou um código de erro.
    /// O stdout é preservado porque linters usam exit code 1 para sinalizar
    /// violações encontradas (não é crash).
    #[error("Process exited with code {exit_code}")]
    ProcessNonZeroExit {
        exit_code: i32,
        stderr: String,
        stdout: Vec<u8>,
    },

    #[error("Execution timed out")]
    Timeout,

}

#[derive(Debug, Clone)]
pub struct SandboxHandle {
    repo_path: PathBuf,
    policy: SandboxPolicy,
    host_write_roots: Vec<PathBuf>,
    active_pids: Arc<Mutex<HashSet<u32>>>,
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
struct WindowsKillOnCloseJob {
    handle: HANDLE,
}

#[cfg(target_os = "windows")]
unsafe impl Send for WindowsKillOnCloseJob {}

#[cfg(target_os = "windows")]
impl Drop for WindowsKillOnCloseJob {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                CloseHandle(self.handle);
            }
            self.handle = std::ptr::null_mut();
        }
    }
}

#[cfg(target_os = "windows")]
fn attach_child_to_kill_on_close_job(
    child: &tokio::process::Child,
) -> Result<WindowsKillOnCloseJob, SandboxError> {
    let process_handle = child.raw_handle().ok_or_else(|| SandboxError::ProcessSpawnFailed {
        reason: "Nao foi possivel capturar raw handle do processo Windows".to_string(),
    })? as HANDLE;

    let job_handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job_handle.is_null() {
        return Err(SandboxError::ProcessSpawnFailed {
            reason: "CreateJobObjectW falhou ao criar Job Object para o sidecar".to_string(),
        });
    }

    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let info_len = u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
        .map_err(|_| SandboxError::ProcessSpawnFailed {
            reason: "Overflow ao calcular tamanho do JOBOBJECT_EXTENDED_LIMIT_INFORMATION".to_string(),
        })?;

    let set_ok = unsafe {
        SetInformationJobObject(
            job_handle,
            JobObjectExtendedLimitInformation,
            &mut info as *mut _ as *mut _,
            info_len,
        )
    };
    if set_ok == 0 {
        unsafe {
            CloseHandle(job_handle);
        }
        return Err(SandboxError::ProcessSpawnFailed {
            reason: "SetInformationJobObject falhou ao ativar KILL_ON_JOB_CLOSE".to_string(),
        });
    }

    let assign_ok = unsafe { AssignProcessToJobObject(job_handle, process_handle) };
    if assign_ok == 0 {
        unsafe {
            CloseHandle(job_handle);
        }
        return Err(SandboxError::ProcessSpawnFailed {
            reason: "AssignProcessToJobObject falhou ao vincular o sidecar ao Job Object".to_string(),
        });
    }

    Ok(WindowsKillOnCloseJob { handle: job_handle })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedCommand {
    program: PathBuf,
    args: Vec<String>,
    env: BTreeMap<String, String>,
}

const IDLE_TIMEOUT_SECS: u64 = 45;
const DEEP_FLOW_IDLE_TIMEOUT_SECS: u64 = 900;
const ABSOLUTE_TIMEOUT_FLOOR_SECS: u64 = 5 * 60;
const PROCESS_WAIT_POLL_INTERVAL_MS: u64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimeoutProfile {
    idle_timeout_secs: u64,
    absolute_timeout_secs: Option<u64>,
}

pub(crate) fn truncated_args_preview<S: AsRef<str>>(args: &[S]) -> Vec<String> {
    const MAX_ARGS_PREVIEW: usize = 3;
    let mut preview = args
        .iter()
        .take(MAX_ARGS_PREVIEW)
        .map(|arg| arg.as_ref().to_string())
        .collect::<Vec<_>>();
    if args.len() > MAX_ARGS_PREVIEW {
        preview.push("<...args omitidos>".to_string());
    }
    preview
}

fn mark_process_activity(last_activity: &Arc<Mutex<Instant>>) {
    let mut guard = last_activity
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = Instant::now();
}

fn idle_elapsed(last_activity: &Arc<Mutex<Instant>>) -> Duration {
    last_activity
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .elapsed()
}

enum ProcessWaitOutcome {
    Exited(std::process::ExitStatus),
    WaitError(std::io::Error),
    IdleTimeout,
    AbsoluteTimeout,
}

fn timeout_profile<S: AsRef<str>>(command: &str, args: &[S], requested_timeout_secs: u64) -> TimeoutProfile {
    match command {
        "cargo" if is_cargo_clippy_invocation(args) => TimeoutProfile {
            idle_timeout_secs: DEEP_FLOW_IDLE_TIMEOUT_SECS,
            absolute_timeout_secs: None,
        },
        "opengrep" | "govulncheck" | "biome" | "oxlint" => TimeoutProfile {
            idle_timeout_secs: DEEP_FLOW_IDLE_TIMEOUT_SECS,
            absolute_timeout_secs: None,
        },
        _ => TimeoutProfile {
            idle_timeout_secs: IDLE_TIMEOUT_SECS,
            absolute_timeout_secs: Some(requested_timeout_secs.max(ABSOLUTE_TIMEOUT_FLOOR_SECS)),
        },
    }
}

fn truncated_env_preview(env: &BTreeMap<String, String>) -> Vec<String> {
    const MAX_ENV_PREVIEW: usize = 3;
    let mut preview = env
        .keys()
        .take(MAX_ENV_PREVIEW)
        .map(|key| format!("{key}=<redacted>"))
        .collect::<Vec<_>>();
    if env.len() > MAX_ENV_PREVIEW {
        preview.push("<...env omitido>".to_string());
    }
    preview
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn executable_names(base_name: &str) -> Vec<String> {
    if cfg!(target_os = "windows") {
        vec![
            format!("{base_name}.exe"),
            format!("{base_name}.cmd"),
            format!("{base_name}.bat"),
            base_name.to_string(),
        ]
    } else {
        vec![base_name.to_string()]
    }
}

fn resolve_from_path(base_name: &str) -> Option<PathBuf> {
    let executable_names = executable_names(base_name);
    let path_var = env::var_os("PATH")?;

    for path_entry in env::split_paths(&path_var) {
        for executable_name in &executable_names {
            let candidate = path_entry.join(executable_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn resolve_local_node_bin(repo_path: &Path, base_name: &str) -> Option<PathBuf> {
    let bin_dir = repo_path.join("node_modules").join(".bin");
    for executable_name in executable_names(base_name) {
        let candidate = bin_dir.join(executable_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn resolve_local_python_bin(repo_path: &Path, base_name: &str) -> Option<PathBuf> {
    let candidates = if cfg!(target_os = "windows") {
        vec![
            repo_path.join(".venv").join("Scripts"),
            repo_path.join("venv").join("Scripts"),
        ]
    } else {
        vec![
            repo_path.join(".venv").join("bin"),
            repo_path.join("venv").join("bin"),
        ]
    };

    for bin_dir in candidates {
        for executable_name in executable_names(base_name) {
            let candidate = bin_dir.join(executable_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn semgrep_support_root(repo_path: &Path) -> PathBuf {
    let repo_name = repo_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("repo");
    repo_path
        .parent()
        .unwrap_or(repo_path)
        .join(".soda_semgrep")
        .join(repo_name)
}

pub(crate) fn sandbox_tool_state_root(repo_path: &Path, tool_name: &str) -> PathBuf {
    let repo_name = repo_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("repo");
    workspace_root()
        .join(".soda_sandbox")
        .join(tool_name)
        .join(repo_name)
}

fn normalize_path_key(path: &Path) -> String {
    let mut value = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = value.strip_prefix("//?/") {
        value = stripped.to_string();
        if let Some(unc_stripped) = value.strip_prefix("UNC/") {
            value = format!("//{unc_stripped}");
        }
    }
    value.to_ascii_lowercase()
}

fn path_is_within_root(candidate: &Path, root: &Path) -> bool {
    let candidate_key = normalize_path_key(candidate);
    let root_key = normalize_path_key(root);
    candidate_key == root_key || candidate_key.starts_with(&(root_key + "/"))
}

fn extract_absolute_arg_paths(args: &[String]) -> Vec<PathBuf> {
    args.iter()
        .filter_map(|arg| {
            let trimmed = arg.trim_matches('"');
            let candidate = PathBuf::from(trimmed);
            candidate.is_absolute().then_some(candidate)
        })
        .collect()
}

fn env_value_to_absolute_path(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim_matches('"');
    let candidate = PathBuf::from(trimmed);
    candidate.is_absolute().then_some(candidate)
}

fn build_host_write_roots(repo_path: &Path, policy: SandboxPolicy) -> Result<Vec<PathBuf>, SandboxError> {
    let mut roots = match policy {
        SandboxPolicy::ReadOnly => Vec::new(),
        SandboxPolicy::ReadWrite => vec![
            semgrep_support_root(repo_path),
            workspace_root().join(".soda_sandbox"),
        ],
    };

    roots.sort();
    roots.dedup();

    for root in &roots {
        std::fs::create_dir_all(root).map_err(|e| SandboxError::PrivilegeError {
            reason: format!("Falha ao preparar raiz de escrita permitida '{}': {}", root.display(), e),
        })?;
    }

    Ok(roots)
}

fn build_semgrep_env(repo_path: &Path) -> BTreeMap<String, String> {
    let sandbox_home = semgrep_support_root(repo_path).join("sandbox");
    let semgrep_dir = sandbox_home.join(".semgrep");

    let _ = std::fs::create_dir_all(&semgrep_dir);

    BTreeMap::from([
        (
            "SEMGREP_LOG_FILE".to_string(),
            semgrep_dir.join("semgrep.log").display().to_string(),
        ),
        (
            "SEMGREP_SETTINGS_FILE".to_string(),
            semgrep_dir.join("settings.yml").display().to_string(),
        ),
    ])
}

fn is_cargo_clippy_invocation<S: AsRef<str>>(args: &[S]) -> bool {
    args.first().map(|arg| arg.as_ref()) == Some("clippy")
}

fn merge_tool_streams(command: &str, stdout: Vec<u8>, stderr: &[u8]) -> Vec<u8> {
    if command != "cppcheck" || stderr.is_empty() {
        return stdout;
    }

    let mut merged = stderr.to_vec();
    if !stdout.is_empty() {
        merged.push(b'\n');
        merged.extend_from_slice(&stdout);
    }
    merged
}

fn is_govulncheck_no_packages_match(command: &str, exit_code: i32, stderr: &[u8]) -> bool {
    if command != "govulncheck" || exit_code != 2 {
        return false;
    }
    String::from_utf8_lossy(stderr)
        .to_ascii_lowercase()
        .contains("no packages matched the provided patterns")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessObservabilityClass {
    Ok,
    InformationalNonZero,
    LethalNonZero,
}

fn classify_process_observability(exit_code: i32, stdout: &[u8]) -> ProcessObservabilityClass {
    if exit_code == 0 {
        ProcessObservabilityClass::Ok
    } else if !stdout.is_empty() {
        ProcessObservabilityClass::InformationalNonZero
    } else {
        ProcessObservabilityClass::LethalNonZero
    }
}

fn persist_semgrep_diagnostics(
    repo_path: &Path,
    resolved: &ResolvedCommand,
    stdout: &[u8],
    stderr: &[u8],
    exit_code: i32,
) -> Option<PathBuf> {
    let diagnostics_dir = semgrep_support_root(repo_path).join("diagnostics");
    std::fs::create_dir_all(&diagnostics_dir).ok()?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let diagnostics_path = diagnostics_dir.join(format!("semgrep-{timestamp}.log"));

    let mut report = String::new();
    report.push_str(&format!("program={}\n", resolved.program.display()));
    report.push_str(&format!("args={:?}\n", resolved.args));
    report.push_str(&format!("exit_code={exit_code}\n"));
    report.push_str(&format!("cwd={}\n", repo_path.display()));
    report.push_str("[env]\n");
    for (key, value) in &resolved.env {
        report.push_str(&format!("{key}={value}\n"));
    }
    report.push_str("\n[stdout]\n");
    report.push_str(&String::from_utf8_lossy(stdout));
    report.push_str("\n\n[stderr]\n");
    report.push_str(&String::from_utf8_lossy(stderr));

    if let Some(log_path) = resolved.env.get("SEMGREP_LOG_FILE") {
        if let Ok(log_content) = std::fs::read_to_string(log_path) {
            report.push_str("\n\n[semgrep_log_file]\n");
            report.push_str(&log_content);
        }
    }

    std::fs::write(&diagnostics_path, report).ok()?;
    Some(diagnostics_path)
}

fn resolve_command(command: &str, args: &[&str], repo_path: &Path) -> Result<ResolvedCommand, SandboxError> {
    match command {
        "pytest" => {
            let program = resolve_local_python_bin(repo_path, "pytest")
                .or_else(|| resolve_from_path("pytest"))
                .unwrap_or_else(|| PathBuf::from(command));
            Ok(ResolvedCommand {
                program,
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                env: BTreeMap::new(),
            })
        }
        "cargo" => {
            let program = resolve_from_path("cargo").unwrap_or_else(|| PathBuf::from(command));
            let mut env = BTreeMap::new();
            env.insert("CARGO_INCREMENTAL".to_string(), "0".to_string());
            env.insert(
                "CARGO_HOME".to_string(),
                sandbox_tool_state_root(repo_path, "cargo-home")
                    .display()
                    .to_string(),
            );
            env.insert(
                "CARGO_REGISTRIES_CRATES_IO_PROTOCOL".to_string(),
                "sparse".to_string(),
            );
            env.insert(
                "CARGO_NET_GIT_FETCH_WITH_CLI".to_string(),
                "false".to_string(),
            );
            let cargo_target_dir = if is_cargo_clippy_invocation(args) {
                sandbox_tool_state_root(repo_path, "cargo-clippy-target")
            } else {
                sandbox_tool_state_root(repo_path, "cargo-target")
            };
            env.insert(
                "CARGO_TARGET_DIR".to_string(),
                cargo_target_dir.display().to_string(),
            );
            Ok(ResolvedCommand {
                program,
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                env,
            })
        }
        "jest" | "vitest" => {
            let program = resolve_local_node_bin(repo_path, command)
                .or_else(|| resolve_from_path(command))
                .unwrap_or_else(|| PathBuf::from(command));
            Ok(ResolvedCommand {
                program,
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                env: BTreeMap::new(),
            })
        }
        "biome" | "oxlint" => {
            let program = resolve_local_node_bin(repo_path, command)
                .or_else(|| resolve_from_path(command))
                .unwrap_or_else(|| PathBuf::from(command));
            Ok(ResolvedCommand {
                program,
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                env: BTreeMap::new(),
            })
        }
        "ruff" | "bandit" => {
            let program = resolve_local_python_bin(repo_path, command)
                .or_else(|| resolve_from_path(command))
                .unwrap_or_else(|| PathBuf::from(command));
            Ok(ResolvedCommand {
                program,
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                env: BTreeMap::new(),
            })
        }
        "mix" => {
            #[cfg(target_os = "windows")]
            {
                let program = resolve_from_path("cmd").unwrap_or_else(|| PathBuf::from("cmd"));
                let mut resolved_args = vec!["/C".to_string(), "mix".to_string()];
                resolved_args.extend(args.iter().map(|arg| (*arg).to_string()));
                Ok(ResolvedCommand {
                    program,
                    args: resolved_args,
                    env: BTreeMap::new(),
                })
            }

            #[cfg(not(target_os = "windows"))]
            {
                let program = resolve_from_path("mix").unwrap_or_else(|| PathBuf::from("mix"));
                Ok(ResolvedCommand {
                    program,
                    args: args.iter().map(|arg| (*arg).to_string()).collect(),
                    env: BTreeMap::new(),
                })
            }
        }
        "semgrep" | "opengrep" | "gh" | "cppcheck" | "sobelow" | "govulncheck" => {
            let env = if command == "semgrep" || command == "opengrep" {
                build_semgrep_env(repo_path)
            } else {
                BTreeMap::new()
            };
            let program = resolve_from_path(command).unwrap_or_else(|| PathBuf::from(command));
            Ok(ResolvedCommand {
                program,
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                env,
            })
        }
        _ => Ok(ResolvedCommand {
            program: PathBuf::from(command),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            env: BTreeMap::new(),
        }),
    }
}

pub(crate) async fn kill_process_tree_by_pid(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let pid = pid.to_string();
        let _ = tokio::task::spawn_blocking(move || {
            let _ = std::process::Command::new("taskkill")
                .args(["/T", "/F", "/PID", &pid])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        })
        .await;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let pid = pid.to_string();
        let _ = tokio::task::spawn_blocking(move || {
            let _ = std::process::Command::new("kill")
                .args(["-9", &pid])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        })
        .await;
    }
}

fn command_requires_orphan_reap(command: &str) -> bool {
    matches!(command, "semgrep" | "opengrep")
}

async fn collect_output_task(task: tokio::task::JoinHandle<Vec<u8>>) -> Vec<u8> {
    match timeout(Duration::from_secs(30), task).await {
        Ok(Ok(buffer)) => buffer,
        _ => Vec::new(),
    }
}

async fn drain_pipe_with_telemetry<R>(
    mut stream: R,
    command: String,
    repo_path: PathBuf,
    pid: u32,
    pipe_name: &'static str,
    last_activity: Arc<Mutex<Instant>>,
) -> Vec<u8>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    use tokio::io::AsyncReadExt;

    const PIPE_CHUNK_SIZE: usize = 64 * 1024;

    let mut buffer = Vec::new();
    let mut chunk = vec![0_u8; PIPE_CHUNK_SIZE];

    loop {
        match stream.read(&mut chunk).await {
            Ok(0) => {
                debug!(
                    command = %command,
                    pid,
                    pipe = pipe_name,
                    total_bytes = buffer.len(),
                    repo_path = %repo_path.display(),
                    "Sandbox: drenagem de pipe concluida"
                );
                break;
            }
            Ok(bytes_read) => {
                buffer.extend_from_slice(&chunk[..bytes_read]);
                mark_process_activity(&last_activity);
                trace!(
                    command = %command,
                    pid,
                    pipe = pipe_name,
                    chunk_bytes = bytes_read,
                    total_bytes = buffer.len(),
                    repo_path = %repo_path.display(),
                    "Sandbox: chunk drenado do pipe"
                );
            }
            Err(e) => {
                warn!(
                    command = %command,
                    pid,
                    pipe = pipe_name,
                    repo_path = %repo_path.display(),
                    error = %e,
                    "Sandbox: falha ao drenar pipe"
                );
                break;
            }
        }
    }

    buffer
}

async fn reap_command_orphans(command: &str, repo_path: &Path) {
    if !command_requires_orphan_reap(command) {
        return;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = repo_path;
    }

    #[cfg(target_os = "windows")]
    {
        let executable_names = match command {
            "semgrep" => vec!["semgrep.exe", "semgrep", "semgrep-core.exe", "semgrep-core"],
            _ => Vec::new(),
        };
        if executable_names.is_empty() {
            return;
        }

        let names_literal = executable_names
            .into_iter()
            .map(|name| format!("'{}'", name.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        let repo_hint = format!("*{}*", repo_path.display()).replace('\'', "''");
        let sandbox_hint = format!("*{}*", semgrep_support_root(repo_path).join("sandbox").display()).replace('\'', "''");
        let script = format!(
            "$ErrorActionPreference = 'SilentlyContinue'; \
             $names = @({names_literal}); \
             Get-CimInstance Win32_Process | Where-Object {{ \
                $names -contains $_.Name -and $_.CommandLine -and ( \
                    $_.CommandLine -like '{repo_hint}' -or \
                    $_.CommandLine -like '{sandbox_hint}' \
                ) \
             }} | ForEach-Object {{ \
                & taskkill.exe /T /F /PID $_.ProcessId 1>$null 2>$null; \
             }}",
            names_literal = names_literal,
            repo_hint = repo_hint,
            sandbox_hint = sandbox_hint,
        );

        let _ = tokio::task::spawn_blocking(move || {
            let _ = std::process::Command::new("powershell.exe")
                .args([
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    &script,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        })
        .await;
    }
}

impl SandboxHandle {
    /// Helper para acessar o Mutex de PIDs de forma segura contra poisoning.
    /// Se o Mutex estiver envenenado (panic em outra thread), recupera o lock
    /// ao invés de propagar o panic — Fail-Safe obrigatório em produção.
    fn lock_pids(&self) -> std::sync::MutexGuard<'_, HashSet<u32>> {
        self.active_pids.lock().unwrap_or_else(|poisoned| {
            // Recupera o guard do Mutex envenenado — os dados internos ainda são válidos.
            // Em produção, o comportamento correto é continuar operando para garantir
            // que o Drop consiga limpar os processos órfãos.
            poisoned.into_inner()
        })
    }

    fn enforce_host_path_policy(&self, resolved: &ResolvedCommand) -> Result<(), SandboxError> {
        let repo_root = &self.repo_path;
        let mut inspected_paths = extract_absolute_arg_paths(&resolved.args);
        inspected_paths.extend(
            resolved
                .env
                .values()
                .filter_map(|value| env_value_to_absolute_path(value)),
        );

        for candidate in inspected_paths {
            let allowed = path_is_within_root(&candidate, repo_root)
                || self
                    .host_write_roots
                    .iter()
                    .any(|root| path_is_within_root(&candidate, root));
            if !allowed {
                return Err(SandboxError::PolicyViolation {
                    detail: format!(
                        "Path absoluto fora da cerca do sandbox: '{}' (repo='{}')",
                        candidate.display(),
                        repo_root.display()
                    ),
                });
            }
        }

        Ok(())
    }

    fn validate_execution_root(&self, execution_root: &Path) -> Result<(), SandboxError> {
        if path_is_within_root(execution_root, &self.repo_path) {
            Ok(())
        } else {
            Err(SandboxError::PolicyViolation {
                detail: format!(
                    "cwd fora da cerca do sandbox: '{}' (repo='{}')",
                    execution_root.display(),
                    self.repo_path.display()
                ),
            })
        }
    }

    async fn execute_with_root(
        &self,
        command: &str,
        args: &[&str],
        timeout_secs: u64,
        execution_root: &Path,
    ) -> Result<Vec<u8>, SandboxError> {
        self.validate_execution_root(execution_root)?;
        let resolved = resolve_command(command, args, execution_root)?;
        self.enforce_host_path_policy(&resolved)?;
        let requested_command = command.to_string();
        debug!(
            command = %requested_command,
            program = %resolved.program.display(),
            args = ?truncated_args_preview(&resolved.args),
            env = ?truncated_env_preview(&resolved.env),
            repo_path = %self.repo_path.display(),
            cwd = %execution_root.display(),
            policy = ?self.policy,
            timeout_secs,
            "Sandbox: iniciando processo efemero"
        );

        let mut process = tokio::process::Command::new(&resolved.program);
        process
            .args(&resolved.args)
            .current_dir(execution_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true);
        if !resolved.env.is_empty() {
            process.envs(&resolved.env);
        }
        let mut child = process
            .spawn()
            .map_err(|e| SandboxError::ProcessSpawnFailed { reason: e.to_string() })?;

        #[cfg(target_os = "windows")]
        let job_guard = Some(attach_child_to_kill_on_close_job(&child)?);
        #[cfg(not(target_os = "windows"))]
        let job_guard: Option<()> = None;

        let pid = child.id().ok_or_else(|| {
            SandboxError::ProcessSpawnFailed { reason: "Não foi possível capturar PID do processo".to_string() }
        })?;

        self.lock_pids().insert(pid);

        let last_activity = Arc::new(Mutex::new(Instant::now()));
        let sys_pid = Pid::from_u32(pid);
        let mut sys = System::new();
        sys.refresh_process(sys_pid);
        let stdout_task = {
            let last_activity = Arc::clone(&last_activity);
            tokio::spawn(drain_pipe_with_telemetry(
                child.stdout.take().ok_or_else(|| {
                    SandboxError::ProcessSpawnFailed { reason: "Não foi possível capturar stdout".to_string() }
                })?,
                requested_command.clone(),
                execution_root.to_path_buf(),
                pid,
                "stdout",
                last_activity,
            ))
        };
        let stderr_task = {
            let last_activity = Arc::clone(&last_activity);
            tokio::spawn(drain_pipe_with_telemetry(
                child.stderr.take().ok_or_else(|| {
                    SandboxError::ProcessSpawnFailed { reason: "Não foi possível capturar stderr".to_string() }
                })?,
                requested_command.clone(),
                execution_root.to_path_buf(),
                pid,
                "stderr",
                last_activity,
            ))
        };
        let timeout_profile = timeout_profile(command, args, timeout_secs);
        let started_at = Instant::now();

        let wait_outcome = loop {
            sys.refresh_process(sys_pid);
            if let Some(process) = sys.process(sys_pid) {
                if process.cpu_usage() > 0.1 {
                    mark_process_activity(&last_activity);
                }
            }
            match child.try_wait() {
                Ok(Some(status)) => break ProcessWaitOutcome::Exited(status),
                Ok(None) => {
                    if idle_elapsed(&last_activity)
                        >= Duration::from_secs(timeout_profile.idle_timeout_secs)
                    {
                        break ProcessWaitOutcome::IdleTimeout;
                    }
                    if let Some(absolute_timeout_secs) = timeout_profile.absolute_timeout_secs {
                        if started_at.elapsed() >= Duration::from_secs(absolute_timeout_secs) {
                            break ProcessWaitOutcome::AbsoluteTimeout;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(PROCESS_WAIT_POLL_INTERVAL_MS)).await;
                }
                Err(e) => break ProcessWaitOutcome::WaitError(e),
            }
        };

        match wait_outcome {
            ProcessWaitOutcome::Exited(status) => {
                let _ = job_guard;
                reap_command_orphans(&requested_command, execution_root).await;
                let stdout_buffer = collect_output_task(stdout_task).await;
                let stderr_buffer = collect_output_task(stderr_task).await;
                self.lock_pids().remove(&pid);
                let exit_code = status.code().unwrap_or(-1);
                let merged_stdout = merge_tool_streams(&requested_command, stdout_buffer, &stderr_buffer);
                if is_govulncheck_no_packages_match(&requested_command, exit_code, &stderr_buffer) {
                    info!(
                        command = %requested_command,
                        pid,
                        exit_code,
                        stdout_bytes = 0,
                        stderr_bytes = stderr_buffer.len(),
                        repo_path = %self.repo_path.display(),
                        cwd = %execution_root.display(),
                        semantic_outcome = "ok",
                        "Sandbox: processo efemero concluido"
                    );
                    return Ok(Vec::new());
                }
                let observability = classify_process_observability(exit_code, &merged_stdout);
                match observability {
                    ProcessObservabilityClass::Ok => {
                        info!(
                            command = %requested_command,
                            pid,
                            exit_code,
                            stdout_bytes = merged_stdout.len(),
                            stderr_bytes = stderr_buffer.len(),
                            repo_path = %self.repo_path.display(),
                            cwd = %execution_root.display(),
                            semantic_outcome = "ok",
                            "Sandbox: processo efemero concluido"
                        );
                    }
                    ProcessObservabilityClass::InformationalNonZero => {
                        warn!(
                            command = %requested_command,
                            pid,
                            exit_code,
                            stdout_bytes = merged_stdout.len(),
                            stderr_bytes = stderr_buffer.len(),
                            repo_path = %self.repo_path.display(),
                            cwd = %execution_root.display(),
                            semantic_outcome = "informational_non_zero",
                            "Sandbox: processo efemero concluido"
                        );
                    }
                    ProcessObservabilityClass::LethalNonZero => {
                        error!(
                            command = %requested_command,
                            pid,
                            exit_code,
                            stdout_bytes = merged_stdout.len(),
                            stderr_bytes = stderr_buffer.len(),
                            repo_path = %self.repo_path.display(),
                            cwd = %execution_root.display(),
                            semantic_outcome = "lethal_non_zero",
                            "Sandbox: processo efemero concluido"
                        );
                    }
                }
                if status.success() {
                    Ok(merged_stdout)
                } else {
                    let mut stderr_msg = String::from_utf8_lossy(&stderr_buffer).trim().to_string();
                    if requested_command == "semgrep" {
                        if let Some(diagnostics_path) = persist_semgrep_diagnostics(
                            execution_root,
                            &resolved,
                            &merged_stdout,
                            &stderr_buffer,
                            exit_code,
                        ) {
                            if stderr_msg.is_empty() {
                                stderr_msg = format!("diagnostics={}", diagnostics_path.display());
                            } else {
                                stderr_msg.push_str(&format!("\ndiagnostics={}", diagnostics_path.display()));
                            }
                        }
                    }
                    Err(SandboxError::ProcessNonZeroExit {
                        exit_code,
                        stderr: stderr_msg,
                        stdout: merged_stdout,
                    })
                }
            }
            ProcessWaitOutcome::WaitError(e) => {
                let _ = job_guard;
                stdout_task.abort();
                stderr_task.abort();
                self.lock_pids().remove(&pid);
                warn!(
                    command = %requested_command,
                    pid,
                    repo_path = %self.repo_path.display(),
                    cwd = %execution_root.display(),
                    error = %e,
                    "Sandbox: erro ao aguardar termino do processo efemero"
                );
                Err(SandboxError::ProcessSpawnFailed { reason: e.to_string() })
            }
            ProcessWaitOutcome::IdleTimeout => {
                warn!(
                    command = %requested_command,
                    pid,
                    repo_path = %self.repo_path.display(),
                    cwd = %execution_root.display(),
                    idle_timeout_secs = timeout_profile.idle_timeout_secs,
                    absolute_timeout_secs = timeout_profile.absolute_timeout_secs.unwrap_or(0),
                    "Sandbox: idle timeout atingido; aniquilando sidecar"
                );
                let _ = child.kill().await;
                let _ = job_guard;
                kill_process_tree_by_pid(pid).await;
                reap_command_orphans(&requested_command, execution_root).await;
                let stdout_buffer = collect_output_task(stdout_task).await;
                let stderr_buffer = collect_output_task(stderr_task).await;
                self.lock_pids().remove(&pid);
                warn!(
                    command = %requested_command,
                    pid,
                    stdout_bytes = stdout_buffer.len(),
                    stderr_bytes = stderr_buffer.len(),
                    repo_path = %self.repo_path.display(),
                    cwd = %execution_root.display(),
                    timeout_kind = "idle",
                    "Sandbox: sidecar aniquilado apos timeout"
                );
                Err(SandboxError::Timeout)
            }
            ProcessWaitOutcome::AbsoluteTimeout => {
                warn!(
                    command = %requested_command,
                    pid,
                    repo_path = %self.repo_path.display(),
                    cwd = %execution_root.display(),
                    idle_timeout_secs = timeout_profile.idle_timeout_secs,
                    absolute_timeout_secs = timeout_profile.absolute_timeout_secs.unwrap_or(0),
                    "Sandbox: absolute timeout atingido; aniquilando sidecar"
                );
                let _ = child.kill().await;
                let _ = job_guard;
                kill_process_tree_by_pid(pid).await;
                reap_command_orphans(&requested_command, execution_root).await;
                let stdout_buffer = collect_output_task(stdout_task).await;
                let stderr_buffer = collect_output_task(stderr_task).await;
                self.lock_pids().remove(&pid);
                warn!(
                    command = %requested_command,
                    pid,
                    stdout_bytes = stdout_buffer.len(),
                    stderr_bytes = stderr_buffer.len(),
                    repo_path = %self.repo_path.display(),
                    cwd = %execution_root.display(),
                    timeout_kind = "absolute",
                    "Sandbox: sidecar aniquilado apos timeout"
                );
                Err(SandboxError::Timeout)
            }
        }
    }

    pub async fn execute(
        &self,
        command: &str,
        args: &[&str],
        timeout_secs: u64,
    ) -> Result<Vec<u8>, SandboxError> {
        self.execute_with_root(command, args, timeout_secs, &self.repo_path)
            .await
    }

    pub async fn execute_in_dir(
        &self,
        command: &str,
        args: &[&str],
        timeout_secs: u64,
        execution_root: &Path,
    ) -> Result<Vec<u8>, SandboxError> {
        self.execute_with_root(command, args, timeout_secs, execution_root)
            .await
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub fn policy(&self) -> SandboxPolicy {
        self.policy
    }
}

// Implementação do Drop RAII à prova de falhas com thread spawn + join para aniquilar processos ativos.
// D1 CORRIGIDO: Usa unwrap_or_else para recuperar de Mutex poisoned ao invés de panic.
impl Drop for SandboxHandle {
    fn drop(&mut self) {
        let pids: Vec<u32> = {
            let guard = self.active_pids.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.iter().copied().collect()
        };

        if !pids.is_empty() {
            // Executa a guilhotina em uma thread dedicada do sistema operacional (PT-3).
            // O join() garante que o Drop é síncrono: o SandboxHandle não é destruído
            // até que todos os processos filhos tenham sido exterminados.
            let _ = std::thread::spawn(move || {
                for pid in pids {
                    #[cfg(target_os = "windows")]
                    {
                        let _ = std::process::Command::new("taskkill")
                            .args(["/T", "/F", "/PID", &pid.to_string()])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status();
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        let _ = std::process::Command::new("kill")
                            .args(["-9", &pid.to_string()])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status();
                    }
                }
            }).join();
        }
    }
}

pub struct SandboxOrchestrator;

impl SandboxOrchestrator {
    pub async fn create(
        repo_path: &RepoPath,
        policy: SandboxPolicy,
    ) -> Result<SandboxHandle, SandboxError> {
        Ok(SandboxHandle {
            repo_path: repo_path.as_ref().to_path_buf(),
            policy,
            host_write_roots: build_host_write_roots(repo_path.as_ref(), policy)?,
            active_pids: Arc::new(Mutex::new(HashSet::new())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::sync::OnceLock;

    static TEST_MUTEX: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

    async fn get_test_mutex() -> &'static tokio::sync::Mutex<()> {
        TEST_MUTEX.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    #[tokio::test]
    async fn test_create_sandbox_success() {
        let _guard = get_test_mutex().await.lock().await;
        
        // Criamos um mock RepoPath usando o diretório temporário nativo do sistema operacional
        let temp_dir = std::env::temp_dir();
        let repo_path = RepoPath(temp_dir);

        let sandbox = SandboxOrchestrator::create(&repo_path, SandboxPolicy::ReadOnly)
            .await
            .expect("Deveria criar sandbox com sucesso");

        assert_eq!(sandbox.policy(), SandboxPolicy::ReadOnly);
        assert_eq!(sandbox.repo_path(), repo_path.as_ref());
    }

    #[tokio::test]
    async fn test_execute_in_sandbox() {
        let _guard = get_test_mutex().await.lock().await;
        
        let temp_dir = std::env::temp_dir();
        let repo_path = RepoPath(temp_dir);

        let sandbox = SandboxOrchestrator::create(&repo_path, SandboxPolicy::ReadOnly)
            .await
            .unwrap();

        // Executa comando básico trivial do próprio sistema para verificar I/O
        #[cfg(target_os = "windows")]
            let output = sandbox.execute("cmd", &["/C", "echo SODA_SANDBOX"], 30).await.unwrap();
        
        #[cfg(not(target_os = "windows"))]
            let output = sandbox.execute("echo", &["SODA_SANDBOX"], 30).await.unwrap();

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.trim().contains("SODA_SANDBOX"));
    }

    #[tokio::test]
    async fn test_read_write_sandbox_creates_allowed_roots() {
        let _guard = get_test_mutex().await.lock().await;

        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("owner").join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        let repo_path = RepoPath(repo_dir.clone());

        let sandbox = SandboxOrchestrator::create(&repo_path, SandboxPolicy::ReadWrite)
            .await
            .expect("sandbox read-write deve ser criado");

        assert_eq!(sandbox.policy(), SandboxPolicy::ReadWrite);
        assert!(repo_dir.parent().unwrap().join(".soda_semgrep").join("repo").exists());
    }

    #[test]
    fn test_resolve_mix_wraps_shell_on_windows() {
        let repo_dir = std::env::temp_dir().join("soda-mix-repo");
        let resolved = resolve_command("mix", &["sobelow", "--format", "json", "--private"], &repo_dir)
            .expect("mix deve ser resolvido");

        #[cfg(target_os = "windows")]
        {
            let program = resolved.program.to_string_lossy().to_ascii_lowercase();
            assert!(program.ends_with("cmd.exe") || program == "cmd");
            assert_eq!(resolved.args, vec!["/C", "mix", "sobelow", "--format", "json", "--private"]);
        }

        #[cfg(not(target_os = "windows"))]
        {
            let program = resolved.program.to_string_lossy().to_ascii_lowercase();
            assert!(program.ends_with("/mix") || program == "mix");
            assert_eq!(resolved.args, vec!["sobelow", "--format", "json", "--private"]);
        }
    }

    #[test]
    fn test_timeout_profile_promotes_deep_flow_tools() {
        let normal = timeout_profile("cppcheck", &["."], 30);
        assert_eq!(normal.idle_timeout_secs, IDLE_TIMEOUT_SECS);
        assert_eq!(normal.absolute_timeout_secs, Some(ABSOLUTE_TIMEOUT_FLOOR_SECS));

        let heavy = timeout_profile("opengrep", &["scan"], 30);
        assert_eq!(heavy.idle_timeout_secs, DEEP_FLOW_IDLE_TIMEOUT_SECS);
        assert_eq!(heavy.absolute_timeout_secs, None);

        let cargo_clippy = timeout_profile("cargo", &["clippy", "--message-format=json"], 30);
        assert_eq!(cargo_clippy.idle_timeout_secs, DEEP_FLOW_IDLE_TIMEOUT_SECS);
        assert_eq!(cargo_clippy.absolute_timeout_secs, None);
    }

    #[test]
    fn test_classify_process_observability_distinguishes_ok_info_and_lethal() {
        assert_eq!(
            classify_process_observability(0, b"{}"),
            ProcessObservabilityClass::Ok
        );
        assert_eq!(
            classify_process_observability(1, b"{\"results\":[]}"),
            ProcessObservabilityClass::InformationalNonZero
        );
        assert_eq!(
            classify_process_observability(101, b""),
            ProcessObservabilityClass::LethalNonZero
        );
    }

    #[test]
    fn test_classify_process_observability_treats_any_stdout_bytes_as_informational_non_zero() {
        assert_eq!(
            classify_process_observability(1, b"\n"),
            ProcessObservabilityClass::InformationalNonZero
        );
    }

    #[test]
    fn test_govulncheck_no_packages_match_is_treated_as_clean() {
        assert!(is_govulncheck_no_packages_match(
            "govulncheck",
            2,
            b"govulncheck: no packages matched the provided patterns",
        ));
        assert!(!is_govulncheck_no_packages_match(
            "govulncheck",
            1,
            b"govulncheck: no packages matched the provided patterns",
        ));
        assert!(!is_govulncheck_no_packages_match(
            "semgrep",
            2,
            b"govulncheck: no packages matched the provided patterns",
        ));
    }

    #[test]
    fn test_resolve_command_injects_isolated_cargo_home_and_sparse_network_guards() {
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("owner").join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();

        let resolved =
            resolve_command("cargo", &["clippy", "--message-format=json"], &repo_dir).unwrap();

        assert_eq!(
            resolved.env.get("CARGO_HOME"),
            Some(&sandbox_tool_state_root(&repo_dir, "cargo-home").display().to_string())
        );
        assert_eq!(
            resolved.env.get("CARGO_TARGET_DIR"),
            Some(&sandbox_tool_state_root(&repo_dir, "cargo-clippy-target").display().to_string())
        );
        assert_eq!(
            resolved.env.get("CARGO_REGISTRIES_CRATES_IO_PROTOCOL"),
            Some(&"sparse".to_string())
        );
        assert_eq!(
            resolved.env.get("CARGO_NET_GIT_FETCH_WITH_CLI"),
            Some(&"false".to_string())
        );
    }

    #[tokio::test]
    async fn test_execute_uses_repo_root_as_cwd() {
        let _guard = get_test_mutex().await.lock().await;

        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("cwd-check");
        std::fs::create_dir_all(&repo_dir).unwrap();
        let repo_path = RepoPath(repo_dir.clone());

        let sandbox = SandboxOrchestrator::create(&repo_path, SandboxPolicy::ReadOnly)
            .await
            .unwrap();

        #[cfg(target_os = "windows")]
        let output = sandbox
            .execute("cmd", &["/C", "cd"], 30)
            .await
            .unwrap();

        #[cfg(not(target_os = "windows"))]
        let output = sandbox.execute("pwd", &[], 30).await.unwrap();

        let output_str = String::from_utf8_lossy(&output).trim().replace('\\', "/");
        let expected = repo_dir.to_string_lossy().replace('\\', "/");
        assert_eq!(output_str, expected);
    }
}
