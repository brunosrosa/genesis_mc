use std::env;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::time::timeout;
use tracing::{info, warn};
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

#[derive(Debug)]
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

fn resolve_code_index_path(repo_path: &Path) -> PathBuf {
    repo_path
        .parent()
        .unwrap_or(repo_path)
        .join(".jcodemunch_index")
}

fn parse_env_assignment(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    let trimmed = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let (name, value) = trimmed.split_once('=')?;
    if name.trim() != key {
        return None;
    }

    let value = value.trim();
    let unquoted = value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|inner| inner.strip_suffix('\'')))
        .unwrap_or(value);

    Some(unquoted.trim().to_string())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

fn read_local_env_var(key: &str) -> Option<String> {
    let candidates = [
        workspace_root().join(".env"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env"),
    ];

    for candidate in candidates {
        let Ok(content) = std::fs::read_to_string(candidate) else {
            continue;
        };
        for line in content.lines() {
            if let Some(value) = parse_env_assignment(line, key) {
                return Some(value);
            }
        }
    }

    None
}

fn resolve_configured_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        Some(candidate)
    } else {
        Some(workspace_root().join(candidate))
    }
}

fn resolve_uvx_path() -> Option<PathBuf> {
    if let Some(value) = env::var_os("SODA_UV_PATH") {
        if let Some(candidate) = resolve_configured_path(&value.to_string_lossy()) {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if let Some(value) = read_local_env_var("SODA_UV_PATH") {
        if let Some(candidate) = resolve_configured_path(&value) {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    let executable_names = if cfg!(target_os = "windows") {
        vec!["uvx.exe", "uvx.cmd", "uvx.bat", "uvx"]
    } else {
        vec!["uvx"]
    };

    if let Some(path_var) = env::var_os("PATH") {
        for path_entry in env::split_paths(&path_var) {
            for executable_name in &executable_names {
                let candidate = path_entry.join(executable_name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    if cfg!(target_os = "windows") {
        let mut well_known = Vec::new();

        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            let base = PathBuf::from(local_app_data);
            well_known.push(
                base.join("Microsoft")
                    .join("WinGet")
                    .join("Packages")
                    .join("astral-sh.uv_Microsoft.Winget.Source_8wekyb3d8bbwe")
                    .join("uvx.exe"),
            );
            well_known.push(base.join("Programs").join("uv").join("uvx.exe"));
        }

        if let Some(app_data) = env::var_os("APPDATA") {
            well_known.push(PathBuf::from(app_data).join("uv").join("uvx.exe"));
        }

        if let Some(user_profile) = env::var_os("USERPROFILE") {
            well_known.push(PathBuf::from(user_profile).join(".local").join("bin").join("uvx.exe"));
        }

        for candidate in well_known {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
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

fn normalize_path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_ascii_lowercase()
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
            resolve_code_index_path(repo_path),
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
        "jcodemunch" | "jcodemunch-mcp" => {
            let uvx_path = resolve_uvx_path().ok_or_else(|| SandboxError::ProcessSpawnFailed {
                reason: "uvx not found; configure SODA_UV_PATH in .env or install uv/uvx in PATH".to_string(),
            })?;

            let mut resolved_args = vec![
                "--from".to_string(),
                "jcodemunch-mcp".to_string(),
                "jcodemunch-mcp".to_string(),
            ];
            resolved_args.extend(args.iter().map(|arg| (*arg).to_string()));

            let mut resolved_env = BTreeMap::new();
            resolved_env.insert(
                "CODE_INDEX_PATH".to_string(),
                resolve_code_index_path(repo_path).display().to_string(),
            );

            Ok(ResolvedCommand {
                program: uvx_path,
                args: resolved_args,
                env: resolved_env,
            })
        }
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
                "CARGO_TARGET_DIR".to_string(),
                workspace_root()
                    .join("src-tauri")
                    .join("target")
                    .join("native-test-list-cache")
                    .display()
                    .to_string(),
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
        "semgrep" | "gh" => {
            let env = if command == "semgrep" {
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
    matches!(command, "jcodemunch" | "jcodemunch-mcp" | "semgrep")
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
                info!(
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
                info!(
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
            "jcodemunch" | "jcodemunch-mcp" => vec!["uvx.exe", "uvx", "jcodemunch-mcp.exe", "jcodemunch-mcp"],
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
        let code_index_hint = format!("*{}*", resolve_code_index_path(repo_path).display()).replace('\'', "''");
        let script = format!(
            "$ErrorActionPreference = 'SilentlyContinue'; \
             $names = @({names_literal}); \
             Get-CimInstance Win32_Process | Where-Object {{ \
                $names -contains $_.Name -and $_.CommandLine -and ( \
                    $_.CommandLine -like '{repo_hint}' -or \
                    $_.CommandLine -like '{sandbox_hint}' -or \
                    $_.CommandLine -like '{code_index_hint}' \
                ) \
             }} | ForEach-Object {{ \
                & taskkill.exe /T /F /PID $_.ProcessId 1>$null 2>$null; \
             }}",
            names_literal = names_literal,
            repo_hint = repo_hint,
            sandbox_hint = sandbox_hint,
            code_index_hint = code_index_hint,
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

    pub async fn execute(
        &self,
        command: &str,
        args: &[&str],
        timeout_secs: u64,
    ) -> Result<Vec<u8>, SandboxError> {
        let resolved = resolve_command(command, args, &self.repo_path)?;
        self.enforce_host_path_policy(&resolved)?;
        let requested_command = command.to_string();
        info!(
            command = %requested_command,
            program = %resolved.program.display(),
            args = ?resolved.args,
            env = ?resolved.env,
            repo_path = %self.repo_path.display(),
            policy = ?self.policy,
            host_write_roots = ?self.host_write_roots,
            timeout_secs,
            "Sandbox: iniciando processo efemero"
        );

        let mut process = tokio::process::Command::new(&resolved.program);
        process
            .args(&resolved.args)
            .current_dir(&self.repo_path)
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

        let stdout_stream = child.stdout.take().ok_or_else(|| {
            SandboxError::ProcessSpawnFailed { reason: "Não foi possível capturar stdout".to_string() }
        })?;
        let stderr_stream = child.stderr.take().ok_or_else(|| {
            SandboxError::ProcessSpawnFailed { reason: "Não foi possível capturar stderr".to_string() }
        })?;

        let stdout_task = tokio::spawn(drain_pipe_with_telemetry(
            stdout_stream,
            requested_command.clone(),
            self.repo_path.clone(),
            pid,
            "stdout",
        ));
        let stderr_task = tokio::spawn(drain_pipe_with_telemetry(
            stderr_stream,
            requested_command.clone(),
            self.repo_path.clone(),
            pid,
            "stderr",
        ));

        let wait_result = timeout(Duration::from_secs(timeout_secs), child.wait()).await;

        match wait_result {
            Ok(Ok(status)) => {
                let _ = job_guard;
                reap_command_orphans(&requested_command, &self.repo_path).await;
                let stdout_buffer = collect_output_task(stdout_task).await;
                let stderr_buffer = collect_output_task(stderr_task).await;
                self.lock_pids().remove(&pid);
                info!(
                    command = %requested_command,
                    pid,
                    exit_code = status.code().unwrap_or(-1),
                    stdout_bytes = stdout_buffer.len(),
                    stderr_bytes = stderr_buffer.len(),
                    repo_path = %self.repo_path.display(),
                    "Sandbox: processo efemero concluido"
                );
                if status.success() {
                    Ok(stdout_buffer)
                } else {
                    let mut stderr_msg = String::from_utf8_lossy(&stderr_buffer).trim().to_string();
                    let exit_code = status.code().unwrap_or(-1);
                    if requested_command == "semgrep" {
                        if let Some(diagnostics_path) = persist_semgrep_diagnostics(
                            &self.repo_path,
                            &resolved,
                            &stdout_buffer,
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
                        stdout: stdout_buffer,
                    })
                }
            }
            Ok(Err(e)) => {
                let _ = job_guard;
                stdout_task.abort();
                stderr_task.abort();
                self.lock_pids().remove(&pid);
                warn!(
                    command = %requested_command,
                    pid,
                    repo_path = %self.repo_path.display(),
                    error = %e,
                    "Sandbox: erro ao aguardar termino do processo efemero"
                );
                Err(SandboxError::ProcessSpawnFailed { reason: e.to_string() })
            }
            Err(_) => {
                warn!(
                    command = %requested_command,
                    pid,
                    repo_path = %self.repo_path.display(),
                    timeout_secs,
                    "Sandbox: timeout atingido; aniquilando sidecar"
                );
                let _ = child.kill().await;
                let _ = job_guard;
                kill_process_tree_by_pid(pid).await;
                reap_command_orphans(&requested_command, &self.repo_path).await;
                let stdout_buffer = collect_output_task(stdout_task).await;
                let stderr_buffer = collect_output_task(stderr_task).await;
                self.lock_pids().remove(&pid);
                warn!(
                    command = %requested_command,
                    pid,
                    stdout_bytes = stdout_buffer.len(),
                    stderr_bytes = stderr_buffer.len(),
                    repo_path = %self.repo_path.display(),
                    "Sandbox: sidecar aniquilado apos timeout"
                );
                Err(SandboxError::Timeout)
            }
        }
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
        assert!(repo_dir.parent().unwrap().join(".jcodemunch_index").exists());
        assert!(repo_dir.parent().unwrap().join(".soda_semgrep").join("repo").exists());
    }

    #[test]
    fn test_parse_env_assignment_reads_soda_uv_path() {
        let parsed = parse_env_assignment(
            r#"SODA_UV_PATH="C:\Tools\uvx.exe""#,
            "SODA_UV_PATH",
        );
        assert_eq!(parsed.as_deref(), Some(r"C:\Tools\uvx.exe"));
    }

    #[test]
    fn test_resolve_configured_path_relative_to_workspace_root() {
        let raw = if cfg!(target_os = "windows") {
            r".soda_scratchpad\bin\uvx.exe"
        } else {
            ".soda_scratchpad/bin/uvx"
        };
        let path = resolve_configured_path(raw)
            .expect("path relativo deve ser resolvido");
        let expected = if cfg!(target_os = "windows") {
            Path::new(".soda_scratchpad").join("bin").join("uvx.exe")
        } else {
            Path::new(".soda_scratchpad").join("bin").join("uvx")
        };
        assert!(path.ends_with(expected));
    }

    #[test]
    fn test_resolve_uvx_path_prefers_process_env() {
        let temp_dir = TempDir::new().unwrap();
        let uvx_path = temp_dir.path().join("uvx.exe");
        std::fs::write(&uvx_path, b"").unwrap();

        std::env::set_var("SODA_UV_PATH", &uvx_path);
        let resolved = resolve_uvx_path();
        std::env::remove_var("SODA_UV_PATH");

        assert_eq!(resolved, Some(uvx_path));
    }
}
