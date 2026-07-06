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
use windows_sys::Win32::Foundation::{CloseHandle, FALSE, HANDLE, INVALID_HANDLE_VALUE, TRUE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::{
    FreeSid,
    PSID, SECURITY_CAPABILITIES,
    ACL,
    DACL_SECURITY_INFORMATION, UNPROTECTED_DACL_SECURITY_INFORMATION,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::Authorization::{
    SetNamedSecurityInfoW, GetNamedSecurityInfoW,
    SetEntriesInAclW, EXPLICIT_ACCESS_W, TRUSTEE_W,
    SE_FILE_OBJECT,
    GRANT_ACCESS, TRUSTEE_IS_SID,
    TRUSTEE_IS_WELL_KNOWN_GROUP,
    NO_MULTIPLE_TRUSTEE,
    ConvertStringSecurityDescriptorToSecurityDescriptorW,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile,
    FILE_FLAG_DELETE_ON_CLOSE, FILE_FLAG_BACKUP_SEMANTICS,
    OPEN_EXISTING,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
    JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Pipes::CreatePipe;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList,
    GetExitCodeProcess, InitializeProcThreadAttributeList,
    UpdateProcThreadAttribute, WaitForSingleObject,
    EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, PROCESS_INFORMATION,
    STARTF_USESTDHANDLES, STARTUPINFOEXW, STARTUPINFOW,
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

    /// Fail-Closed: a injeção de ACL NTFS falhou para o AppContainer SID.
    /// O processo filho NÃO é spawnado para evitar operação cega sem permissões.
    #[error("AppContainer ACL injection failed: {detail}")]
    AclInjectionFailed { detail: String },

    /// O perfil AppContainer não pôde ser criado ou configurado.
    #[error("AppContainer setup failed: {detail}")]
    AppContainerSetupFailed { detail: String },
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

// ═══════════════════════════════════════════════════════════════════════════
// GAIOLA DE SILÍCIO — AppContainer / LPAC
// Isolamento real de Kernel para sidecars efêmeros do SODA.
// Arquitetura: AppContainerProfile (Drop → higiene do Registro) +
//              STARTUPINFOEX (injeção de credenciais antes do spawn) +
//              ACLs NTFS Fail-Closed + DELETE_ON_CLOSE handle efêmero.
// ═══════════════════════════════════════════════════════════════════════════

/// Converte uma &str UTF-8 para um Vec<u16> null-terminated (PCWSTR compatível).
#[cfg(target_os = "windows")]
fn str_to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Converte um Path para Vec<u16> null-terminated.
#[cfg(target_os = "windows")]
fn path_to_wide(path: &std::path::Path) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Obtém o último erro Win32 como string legível.
#[cfg(target_os = "windows")]
fn last_win32_error() -> String {
    let code = unsafe { windows_sys::Win32::Foundation::GetLastError() };
    format!("Win32 error code: {code:#010x}")
}

/// Perfil AppContainer com Drop automático para higiene do Registro do Windows.
/// O Registro fica sujo se `DeleteAppContainerProfile` não for chamado.
/// Esta struct garante a chamada rigorosa via trait Drop, mesmo em panic.
#[cfg(target_os = "windows")]
#[derive(Debug)]
pub struct AppContainerProfile {
    /// SID alocado pelo kernel via CreateAppContainerProfile.
    /// DEVE ser liberado com FreeSid no Drop.
    sid: PSID,
    /// Nome do perfil em UTF-16 null-terminated, usado para DeleteAppContainerProfile.
    name_wide: Vec<u16>,
}

#[cfg(target_os = "windows")]
// SAFETY: PSID é um ponteiro opaco gerenciado exclusivamente por esta struct.
// Não há acesso concorrente: a struct é movida para spawn_blocking e
// volta ao SandboxHandle após conclusão.
unsafe impl Send for AppContainerProfile {}

#[cfg(target_os = "windows")]
impl Drop for AppContainerProfile {
    fn drop(&mut self) {
        // SAFETY: Garantido pela invariante de que sid foi retornado por
        // CreateAppContainerProfile e ainda não foi liberado.
        unsafe {
            // Higiene do Registro: remove o perfil para evitar Registry Leak.
            // Ignora erros no Drop — não há alternativa segura.
            let _hr = DeleteAppContainerProfile(self.name_wide.as_ptr());
            if !self.sid.is_null() {
                FreeSid(self.sid);
                self.sid = std::ptr::null_mut();
            }
        }
    }
}

/// Cria um perfil AppContainer com nome único baseado no timestamp.
/// Retorna `Err(AppContainerSetupFailed)` se CreateAppContainerProfile falhar.
#[cfg(target_os = "windows")]
fn create_appcontainer_profile(
    container_name: &str,
) -> Result<AppContainerProfile, SandboxError> {
    let name_wide = str_to_wide(container_name);
    let display_wide = str_to_wide(&format!("SODA Sidecar: {container_name}"));
    let desc_wide = str_to_wide("SODA ephemeral AppContainer for sidecar isolation");

    let mut sid: PSID = std::ptr::null_mut();
    let hr = unsafe {
        CreateAppContainerProfile(
            name_wide.as_ptr(),
            display_wide.as_ptr(),
            desc_wide.as_ptr(),
            // Sem capabilities adicionais (LPAC = perfil mais restrito).
            // Para adicionar capabilities futuras, passe um slice de SID_AND_ATTRIBUTES aqui.
            std::ptr::null(),
            0,
            &mut sid,
        )
    };

    // HRESULT: bit 31 = sinal de erro. 0x800700B7 = HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS)
    // Perfil existente é recuperável: basta derivar o SID pelo nome.
    let sid = if hr == 0x800700B7_u32 as i32 {
        // Perfil já existe; derivamos o SID pelo nome canônico.
        let mut existing_sid: PSID = std::ptr::null_mut();
        let hr2 = unsafe {
            windows_sys::Win32::Security::Isolation::DeriveAppContainerSidFromAppContainerName(
                name_wide.as_ptr(),
                &mut existing_sid,
            )
        };
        if hr2 < 0 || existing_sid.is_null() {
            return Err(SandboxError::AppContainerSetupFailed {
                detail: format!(
                    "Perfil '{container_name}' existente, mas DeriveAppContainerSidFromAppContainerName falhou: {hr2:#010x}"
                ),
            });
        }
        existing_sid
    } else if hr < 0 || sid.is_null() {
        return Err(SandboxError::AppContainerSetupFailed {
            detail: format!(
                "CreateAppContainerProfile falhou para '{container_name}': HRESULT={hr:#010x}"
            ),
        });
    } else {
        sid
    };

    Ok(AppContainerProfile { sid, name_wide })
}

/// Muro do NTFS — Fail-Closed.
/// Adiciona uma entrada de permissão para o AppContainer SID no DACL do diretório.
/// Se SetNamedSecurityInfoW falhar, retorna Err(AclInjectionFailed) e o processo
/// NÃO é spawnado (princípio SODA de Zero-Falhas-Silenciosas).
///
/// `access_mask`: use combinações de FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE.
#[cfg(target_os = "windows")]
fn grant_ntfs_acl(
    path: &std::path::Path,
    sid: PSID,
    access_mask: u32,
) -> Result<(), SandboxError> {
    let path_wide = path_to_wide(path);

    // 1. Obtem o DACL existente para não sobrescrever permissões do host.
    let mut existing_dacl: *mut ACL = std::ptr::null_mut();
    let mut sd_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let get_result = unsafe {
        GetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut existing_dacl,
            std::ptr::null_mut(),
            &mut sd_ptr as *mut _ as *mut *mut _,
        )
    };

    // GetNamedSecurityInfoW retorna um código Win32 (0 = sucesso), não HRESULT.
    if get_result != 0 {
        return Err(SandboxError::AclInjectionFailed {
            detail: format!(
                "GetNamedSecurityInfoW falhou para '{}': Win32={get_result:#010x}",
                path.display()
            ),
        });
    }

    // 2. Monta a nova entrada de acesso para o AppContainer SID.
    // TRUSTEE_W.ptstrName é uma union com pSid — cast seguro conforme Win32 doc.
    let mut ea = EXPLICIT_ACCESS_W {
        grfAccessPermissions: access_mask,
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: 3u32, // CONTAINER_INHERIT_ACE | OBJECT_INHERIT_ACE
        Trustee: TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_WELL_KNOWN_GROUP,
            // SAFETY: sid é válido durante a chamada. Cast PSID -> PWSTR é o
            // idioma Win32 padrão para TRUSTEE_FORM = TRUSTEE_IS_SID.
            ptstrName: sid as windows_sys::core::PWSTR,
        },
    };

    // 3. Mescla a nova entrada com o DACL existente.
    let mut new_dacl: *mut ACL = std::ptr::null_mut();
    let merge_result = unsafe {
        SetEntriesInAclW(
            1,
            &mut ea,
            // Passa o existing_dacl para MESCLAR (não substituir).
            if existing_dacl.is_null() { std::ptr::null_mut() } else { existing_dacl as *mut _ },
            &mut new_dacl,
        )
    };

    // Libera o security descriptor alocado por GetNamedSecurityInfoW.
    if !sd_ptr.is_null() {
        unsafe { windows_sys::Win32::Foundation::LocalFree(sd_ptr as *mut _); }
    }

    if merge_result != 0 || new_dacl.is_null() {
        return Err(SandboxError::AclInjectionFailed {
            detail: format!(
                "SetEntriesInAclW falhou para '{}': Win32={merge_result:#010x}",
                path.display()
            ),
        });
    }

    // 4. Aplica o novo DACL mesclado. UNPROTECTED_DACL preserva herança do pai.
    let apply_result = unsafe {
        SetNamedSecurityInfoW(
            // SAFETY: path_wide é null-terminated UTF-16 válido.
            path_wide.as_ptr() as *mut u16,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | UNPROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_dacl,
            std::ptr::null_mut(),
        )
    };

    // Libera o novo ACL alocado por SetEntriesInAclW.
    unsafe { windows_sys::Win32::Foundation::LocalFree(new_dacl as *mut _); }

    if apply_result != 0 {
        return Err(SandboxError::AclInjectionFailed {
            detail: format!(
                "SetNamedSecurityInfoW falhou para '{}': Win32={apply_result:#010x}",
                path.display()
            ),
        });
    }

    Ok(())
}

/// Wrapper seguro de thread para transportar HANDLEs Win32 através de limites de await.
/// HANDLE é definido como *mut c_void em windows-sys, o que não implementa Send.
#[cfg(target_os = "windows")]
#[derive(Debug)]
struct SendHandle(HANDLE);

#[cfg(target_os = "windows")]
unsafe impl Send for SendHandle {}

#[cfg(target_os = "windows")]
unsafe impl Sync for SendHandle {}

/// Abre o diretório com FILE_FLAG_DELETE_ON_CLOSE.
/// O handle retornado DEVE ser armazenado na struct de Sandbox e fechado via CloseHandle no Drop.
/// Quando o handle fecha, o NTFS remove o diretório automaticamente — "Evaporação de Handle".
#[cfg(target_os = "windows")]
fn open_dir_delete_on_close(path: &std::path::Path) -> Result<HANDLE, SandboxError> {
    let path_wide = path_to_wide(path);
    let handle = unsafe {
        CreateFileW(
            path_wide.as_ptr(),
            // GENERIC_READ é necessário para abrir o diretório sem acesso de escrita exclusivo.
            0x8000_0000u32, // GENERIC_READ
            // FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE
            0x0001 | 0x0002 | 0x0004,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_DELETE_ON_CLOSE | FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(SandboxError::AppContainerSetupFailed {
            detail: format!(
                "Falha ao abrir diretório efêmero com DELETE_ON_CLOSE: '{}': {}",
                path.display(),
                last_win32_error()
            ),
        });
    }
    Ok(handle)
}

/// Isenção de Loopback (Loopback Exemption) para o AppContainer.
/// Permite que o sidecar acesse Named Pipes do host via loopback (127.0.0.1 / ::1).
/// Usa CheckNetIsolation.exe — o utilitário oficial da Microsoft para este fim.
///
/// Esta função é best-effort: se CheckNetIsolation não estiver disponível,
/// emite um warning mas não falha (a conectividade real será testada no runtime).
///
/// SODA IPC — PLACEHOLDER DACL DOS NAMED PIPES:
/// O processo pai (SODA Gateway) precisa adicionar a seguinte ACE ao DACL
/// de cada Named Pipe que o sidecar consumirá:
///   Trustee: "ALL APPLICATION PACKAGES" (SID: S-1-15-2-1)
///   Permissões: GENERIC_READ | GENERIC_WRITE
/// Use SetSecurityInfo(pipe_handle, SE_KERNEL_OBJECT, DACL_SECURITY_INFORMATION, ...)
/// no momento da criação do pipe, antes de abrir a conexão do sidecar.
#[cfg(target_os = "windows")]
fn set_loopback_exemption(profile_name: &str) -> bool {
    // CheckNetIsolation.exe LoopbackExempt -a -n=<nome_do_perfil>
    let result = std::process::Command::new("CheckNetIsolation.exe")
        .args(["LoopbackExempt", "-a", &format!("-n={profile_name}")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match result {
        Ok(status) => status.success(),
        Err(e) => {
            warn!(
                profile_name,
                error = %e,
                "AppContainer: CheckNetIsolation não disponível; loopback IPC pode falhar"
            );
            false
        }
    }
}

/// Resultado de um spawn em AppContainer — contém stdout/stderr coletados
/// e o código de saída. O AppContainerProfile e o handle efêmero são
/// gerenciados externamente no SandboxHandle.
#[cfg(target_os = "windows")]
struct AppContainerSpawnResult {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: i32,
}

/// Spawn de processo dentro de AppContainer via CreateProcessW + STARTUPINFOEX.
/// Executa COMPLETAMENTE em contexto bloqueante (spawn_blocking do caller).
///
/// Garante:
/// - Credenciais AppContainer via PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES
/// - Pipes anônimos para stdout/stderr (coleta síncrona)
/// - Job Object KILL_ON_JOB_CLOSE para morte atômica
/// - WaitForSingleObject com timeout em ms
///
/// # Safety
/// Todos os ponteiros Win32 têm lifetime controlado por escopo RAII nesta função.
#[cfg(target_os = "windows")]
fn spawn_in_appcontainer_blocking(
    program: &std::path::Path,
    args: &[String],
    env: &std::collections::BTreeMap<String, String>,
    cwd: &std::path::Path,
    profile: &AppContainerProfile,
    timeout_ms: u32,
) -> Result<AppContainerSpawnResult, SandboxError> {
    // ── 1. Monta a linha de comando UTF-16 ────────────────────────────────────
    // Escapa argumentos conforme a regra canônica do Win32 para CreateProcessW.
    let escape_cmd_arg = |arg: &str| -> String {
        if arg.is_empty() {
            return "\"\"".to_string();
        }
        if !arg.contains(' ') && !arg.contains('\t') && !arg.contains('\"') && !arg.contains('\\') {
            return arg.to_string();
        }
        let mut res = String::new();
        res.push('"');
        let mut backslashes = 0;
        for c in arg.chars() {
            match c {
                '\\' => backslashes += 1,
                '"' => {
                    for _ in 0..backslashes * 2 {
                        res.push('\\');
                    }
                    backslashes = 0;
                    res.push_str("\\\"");
                }
                _ => {
                    for _ in 0..backslashes {
                        res.push('\\');
                    }
                    backslashes = 0;
                    res.push(c);
                }
            }
        }
        for _ in 0..backslashes * 2 {
            res.push('\\');
        }
        res.push('"');
        res
    };

    let cmd_str = std::iter::once(escape_cmd_arg(&program.to_string_lossy()))
        .chain(args.iter().map(|a| escape_cmd_arg(a)))
        .collect::<Vec<String>>()
        .join(" ");
    let mut cmd_wide: Vec<u16> = str_to_wide(&cmd_str);
    let cwd_wide = path_to_wide(cwd);

    // ── 2. Bloco de ambiente UTF-16 null-null ─────────────────────────────────
    // Formato: KEY=VALUE\0KEY=VALUE\0\0
    let env_block: Vec<u16> = {
        let mut block: Vec<u16> = Vec::new();
        // Inclui o ambiente do processo pai mais as variáveis injetadas.
        let mut merged_env: std::collections::BTreeMap<String, String> =
            std::env::vars().collect();
        merged_env.extend(env.clone());
        for (k, v) in &merged_env {
            // Ignora variáveis vazias ou chaves internas do Windows que começam com '='.
            if k.is_empty() || k.starts_with('=') {
                continue;
            }
            block.extend(str_to_wide(&format!("{k}={v}")).into_iter());
        }
        block.push(0); // bloco termina com duplo null
        block
    };

    // ── 3. Pipes anônimos para stdout/stderr ──────────────────────────────────
    // Herança seletiva: apenas os handles de escrita são herdados pelo filho.
    // O descritor de segurança deve possuir uma DACL que conceda permissões de escrita/leitura
    // para "Everyone" (WD) e "ALL APPLICATION PACKAGES" (S-1-15-2-1), permitindo que o
    // subprocesso rodando no AppContainer restrito escreva nos handles de pipe herdados.
    let mut sa: windows_sys::Win32::Security::SECURITY_ATTRIBUTES =
        unsafe { std::mem::zeroed() };
    sa.nLength = size_of::<windows_sys::Win32::Security::SECURITY_ATTRIBUTES>() as u32;
    sa.bInheritHandle = TRUE;

    let sddl = str_to_wide("D:(A;;GA;;;WD)(A;;GA;;;S-1-15-2-1)");
    let mut sd: *mut std::ffi::c_void = std::ptr::null_mut();
    let mut sd_size: u32 = 0;
    let sd_ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1, // SDDL_REVISION_1
            &mut sd as *mut _ as *mut *mut std::ffi::c_void,
            &mut sd_size,
        )
    };
    if sd_ok == FALSE || sd.is_null() {
        return Err(SandboxError::AppContainerSetupFailed {
            detail: format!(
                "ConvertStringSecurityDescriptorToSecurityDescriptorW falhou: {}",
                last_win32_error()
            ),
        });
    }
    sa.lpSecurityDescriptor = sd;

    let (mut stdout_read, mut stdout_write) = (INVALID_HANDLE_VALUE, INVALID_HANDLE_VALUE);
    let (mut stderr_read, mut stderr_write) = (INVALID_HANDLE_VALUE, INVALID_HANDLE_VALUE);

    let create_stdout_ok = unsafe { CreatePipe(&mut stdout_read, &mut stdout_write, &sa, 0) };
    let create_stderr_ok = unsafe { CreatePipe(&mut stderr_read, &mut stderr_write, &sa, 0) };

    // Libera a memória do descritor de segurança alocado dinamicamente pelo sistema.
    unsafe {
        windows_sys::Win32::Foundation::LocalFree(sd);
    }

    if create_stdout_ok == FALSE {
        if create_stderr_ok == TRUE {
            unsafe { CloseHandle(stderr_read); CloseHandle(stderr_write); }
        }
        return Err(SandboxError::ProcessSpawnFailed {
            reason: format!("CreatePipe (stdout) falhou: {}", last_win32_error()),
        });
    }
    if create_stderr_ok == FALSE {
        unsafe { CloseHandle(stdout_read); CloseHandle(stdout_write); }
        return Err(SandboxError::ProcessSpawnFailed {
            reason: format!("CreatePipe (stderr) falhou: {}", last_win32_error()),
        });
    }

    // Desabilita herança nos lados de leitura (o pai não herda seus próprios pipes).
    unsafe {
        windows_sys::Win32::Foundation::SetHandleInformation(
            stdout_read, 0x1 /*HANDLE_FLAG_INHERIT*/, 0
        );
        windows_sys::Win32::Foundation::SetHandleInformation(
            stderr_read, 0x1 /*HANDLE_FLAG_INHERIT*/, 0
        );
    }

    // ── 4. Lista de atributos do processo (PROC_THREAD_ATTRIBUTE_LIST) ────────
    let mut attr_list_size: usize = 0;
    // Segunda chamada: calcula o tamanho necessário para 2 atributos:
    // 1. SECURITY_CAPABILITIES (AppContainer SID)
    // 2. HANDLE_LIST (Lista restrita de handles herdáveis)
    unsafe {
        InitializeProcThreadAttributeList(
            std::ptr::null_mut(), 2, 0, &mut attr_list_size,
        );
    }
    let mut attr_list_buf: Vec<u8> = vec![0u8; attr_list_size];
    let attr_list: LPPROC_THREAD_ATTRIBUTE_LIST = attr_list_buf.as_mut_ptr() as *mut _;

    let init_ok = unsafe {
        InitializeProcThreadAttributeList(attr_list, 2, 0, &mut attr_list_size)
    };
    if init_ok == FALSE {
        unsafe {
            CloseHandle(stdout_read); CloseHandle(stdout_write);
            CloseHandle(stderr_read); CloseHandle(stderr_write);
        }
        return Err(SandboxError::AppContainerSetupFailed {
            detail: format!("InitializeProcThreadAttributeList falhou: {}", last_win32_error()),
        });
    }

    // ── 5. Injeta SECURITY_CAPABILITIES com o SID do AppContainer ─────────────
    let mut caps = SECURITY_CAPABILITIES {
        AppContainerSid: profile.sid,
        // Sem capabilities extras: LPAC puro (Less Privileged AppContainer).
        Capabilities: std::ptr::null_mut(),
        CapabilityCount: 0,
        Reserved: 0,
    };

    let update_ok = unsafe {
        UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            &mut caps as *mut SECURITY_CAPABILITIES as *mut _,
            size_of::<SECURITY_CAPABILITIES>(),
            std::ptr::null_mut(),
            std::ptr::null(),
        )
    };
    if update_ok == FALSE {
        unsafe {
            DeleteProcThreadAttributeList(attr_list);
            CloseHandle(stdout_read); CloseHandle(stdout_write);
            CloseHandle(stderr_read); CloseHandle(stderr_write);
        }
        return Err(SandboxError::AppContainerSetupFailed {
            detail: format!("UpdateProcThreadAttribute (security caps) falhou: {}", last_win32_error()),
        });
    }

    // ── 5.1 Injeta PROC_THREAD_ATTRIBUTE_HANDLE_LIST para restringir herança ────
    // Se bInheritHandles = TRUE no CreateProcessW, o Windows restringe a herança apenas
    // para esta lista explícita, evitando falhas de Spawn no AppContainer.
    const PROC_THREAD_ATTRIBUTE_HANDLE_LIST: usize = 0x00020002;
    let mut handles_to_inherit = [stdout_write, stderr_write];
    let update_handles_ok = unsafe {
        UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            handles_to_inherit.as_mut_ptr() as *mut _,
            handles_to_inherit.len() * size_of::<HANDLE>(),
            std::ptr::null_mut(),
            std::ptr::null(),
        )
    };
    if update_handles_ok == FALSE {
        unsafe {
            DeleteProcThreadAttributeList(attr_list);
            CloseHandle(stdout_read); CloseHandle(stdout_write);
            CloseHandle(stderr_read); CloseHandle(stderr_write);
        }
        return Err(SandboxError::AppContainerSetupFailed {
            detail: format!("UpdateProcThreadAttribute (handles list) falhou: {}", last_win32_error()),
        });
    }

    // ── 6. Monta STARTUPINFOEXW ───────────────────────────────────────────────
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    si.StartupInfo.hStdInput = std::ptr::null_mut();
    si.StartupInfo.hStdOutput = stdout_write;
    si.StartupInfo.hStdError = stderr_write;
    si.lpAttributeList = attr_list;

    // ── 7. Spawn via CreateProcessW ───────────────────────────────────────────
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let create_ok = unsafe {
        CreateProcessW(
            std::ptr::null(),
            cmd_wide.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            TRUE, // bInheritHandles: herda stdout_write e stderr_write
            EXTENDED_STARTUPINFO_PRESENT, // Omitimos CREATE_UNICODE_ENVIRONMENT quando passamos null no env
            std::ptr::null(), // Herda o ambiente padrão do pai diretamente via kernel
            cwd_wide.as_ptr(),
            // SAFETY: STARTUPINFOEXW começa com STARTUPINFOW; cast é seguro conforme Win32 ABI.
            &si.StartupInfo as *const STARTUPINFOW,
            &mut pi,
        )
    };

    // Libera a lista de atributos imediatamente após CreateProcessW.
    unsafe { DeleteProcThreadAttributeList(attr_list); }
    // Fecha os handles de escrita no lado pai (o filho tem sua própria cópia herdada).
    unsafe { CloseHandle(stdout_write); CloseHandle(stderr_write); }

    if create_ok == FALSE || pi.hProcess.is_null() {
        unsafe {
            CloseHandle(stdout_read);
            CloseHandle(stderr_read);
        }
        return Err(SandboxError::ProcessSpawnFailed {
            reason: format!("CreateProcessW (AppContainer) falhou: {}", last_win32_error()),
        });
    }

    // O thread filho foi criado em estado normal (sem CREATE_SUSPENDED);
    // fechar o handle do thread é seguro aqui.
    unsafe { CloseHandle(pi.hThread); }

    // ── 8. Vincula ao Job Object KILL_ON_JOB_CLOSE ────────────────────────────
    let job_result = create_kill_on_close_job_for_handle(pi.hProcess);
    // Job Object é best-effort para AppContainer (o AC já garante morte em cascata).
    // Registra warning mas não aborta.
    let _job_guard = match job_result {
        Ok(j) => Some(j),
        Err(e) => {
            warn!(error = %e, "AppContainer: Job Object opcional nao foi vinculado");
            None
        }
    };

    // ── 9. Coleta output e aguarda com timeout ────────────────────────────────
    const CHUNK: usize = 64 * 1024;
    let mut stdout_buf: Vec<u8> = Vec::new();
    let mut stderr_buf: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; CHUNK];
    let mut bytes_read: u32;

    // Drena stdout e stderr em loop usando WaitForSingleObject com polling.
    // Note: reads on pipes retornam ERROR_BROKEN_PIPE quando o filho fecha o handle.
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms as u64);
    loop {
        let remaining_ms = deadline
            .checked_duration_since(std::time::Instant::now())
            .map(|d| d.as_millis() as u32)
            .unwrap_or(0);

        if remaining_ms == 0 {
            // Timeout: mata o processo e retorna erro.
            unsafe {
                windows_sys::Win32::System::Threading::TerminateProcess(pi.hProcess, 1);
                WaitForSingleObject(pi.hProcess, 1000);
                CloseHandle(pi.hProcess);
                CloseHandle(stdout_read);
                CloseHandle(stderr_read);
            }
            return Err(SandboxError::Timeout);
        }

        // Polling de 250ms: balanceia responsividade vs CPU.
        let wait_result = unsafe {
            WaitForSingleObject(pi.hProcess, 250.min(remaining_ms))
        };

        // Drena o que há disponível em stdout (não-bloqueante via PeekNamedPipe).
        loop {
            let mut bytes_avail: u32 = 0;
            let peek_ok = unsafe {
                windows_sys::Win32::System::Pipes::PeekNamedPipe(
                    stdout_read,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut bytes_avail,
                    std::ptr::null_mut(),
                )
            };
            if peek_ok == FALSE || bytes_avail == 0 {
                break;
            }

            let to_read = (bytes_avail as usize).min(CHUNK);
            bytes_read = 0;
            let ok = unsafe {
                ReadFile(
                    stdout_read,
                    chunk.as_mut_ptr() as *mut _,
                    to_read as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            };
            if ok == FALSE || bytes_read == 0 {
                break;
            }
            stdout_buf.extend_from_slice(&chunk[..bytes_read as usize]);
        }

        // Drena o que há disponível em stderr (não-bloqueante via PeekNamedPipe).
        loop {
            let mut bytes_avail: u32 = 0;
            let peek_ok = unsafe {
                windows_sys::Win32::System::Pipes::PeekNamedPipe(
                    stderr_read,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut bytes_avail,
                    std::ptr::null_mut(),
                )
            };
            if peek_ok == FALSE || bytes_avail == 0 {
                break;
            }

            let to_read = (bytes_avail as usize).min(CHUNK);
            bytes_read = 0;
            let ok = unsafe {
                ReadFile(
                    stderr_read,
                    chunk.as_mut_ptr() as *mut _,
                    to_read as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            };
            if ok == FALSE || bytes_read == 0 {
                break;
            }
            stderr_buf.extend_from_slice(&chunk[..bytes_read as usize]);
        }

        // WAIT_OBJECT_0 = 0x0000_0000 = processo terminou.
        if wait_result == 0x0000_0000u32 {
            break;
        }
    }

    // Leitura final após término (garante que não deixamos bytes nos pipes).
    bytes_read = 0;
    loop {
        let ok = unsafe {
            ReadFile(
                stdout_read, chunk.as_mut_ptr() as *mut _, CHUNK as u32,
                &mut bytes_read, std::ptr::null_mut(),
            )
        };
        if ok == FALSE || bytes_read == 0 { break; }
        stdout_buf.extend_from_slice(&chunk[..bytes_read as usize]);
    }
    bytes_read = 0;
    loop {
        let ok = unsafe {
            ReadFile(
                stderr_read, chunk.as_mut_ptr() as *mut _, CHUNK as u32,
                &mut bytes_read, std::ptr::null_mut(),
            )
        };
        if ok == FALSE || bytes_read == 0 { break; }
        stderr_buf.extend_from_slice(&chunk[..bytes_read as usize]);
    }

    // ── 10. Coleta exit code e limpa handles ──────────────────────────────────
    let mut exit_code: u32 = 0;
    unsafe {
        GetExitCodeProcess(pi.hProcess, &mut exit_code);
        CloseHandle(pi.hProcess);
        CloseHandle(stdout_read);
        CloseHandle(stderr_read);
    }

    Ok(AppContainerSpawnResult {
        stdout: stdout_buf,
        stderr: stderr_buf,
        exit_code: exit_code as i32,
    })
}

/// Helper interno: cria um Job Object KILL_ON_JOB_CLOSE para um handle de processo.
/// Diferente de `attach_child_to_kill_on_close_job` que recebe `tokio::process::Child`.
#[cfg(target_os = "windows")]
fn create_kill_on_close_job_for_handle(process_handle: HANDLE) -> Result<WindowsKillOnCloseJob, SandboxError> {
    let job_handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job_handle.is_null() {
        return Err(SandboxError::ProcessSpawnFailed {
            reason: format!("CreateJobObjectW falhou para AppContainer: {}", last_win32_error()),
        });
    }
    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    let set_ok = unsafe {
        SetInformationJobObject(
            job_handle, JobObjectExtendedLimitInformation,
            &mut info as *mut _ as *mut _,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if set_ok == 0 {
        unsafe { CloseHandle(job_handle); }
        return Err(SandboxError::ProcessSpawnFailed {
            reason: "SetInformationJobObject falhou para AppContainer Job Object".to_string(),
        });
    }
    let assign_ok = unsafe { AssignProcessToJobObject(job_handle, process_handle) };
    if assign_ok == 0 {
        unsafe { CloseHandle(job_handle); }
        return Err(SandboxError::ProcessSpawnFailed {
            reason: "AssignProcessToJobObject falhou para AppContainer".to_string(),
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
        "cargo" if is_cargo_sast_invocation(args) => TimeoutProfile {
            idle_timeout_secs: DEEP_FLOW_IDLE_TIMEOUT_SECS,
            absolute_timeout_secs: None,
        },
        "opengrep" | "govulncheck" | "biome" | "oxlint" | "cppcheck" => TimeoutProfile {
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

fn is_cargo_sast_invocation<S: AsRef<str>>(args: &[S]) -> bool {
    matches!(
        args.first().map(|value| value.as_ref()),
        Some("clippy" | "fetch" | "metadata")
    )
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
            let cargo_target_dir = if is_cargo_sast_invocation(args) {
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
        #[cfg(target_os = "windows")]
        {
            self.execute_in_appcontainer(command, args, timeout_secs).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.execute_with_root(command, args, timeout_secs, &self.repo_path)
                .await
        }
    }

    pub async fn execute_in_dir(
        &self,
        command: &str,
        args: &[&str],
        timeout_secs: u64,
        execution_root: &Path,
    ) -> Result<Vec<u8>, SandboxError> {
        #[cfg(target_os = "windows")]
        {
            self.execute_in_appcontainer_in_dir(command, args, timeout_secs, execution_root).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.execute_with_root(command, args, timeout_secs, execution_root)
                .await
        }
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    pub fn policy(&self) -> SandboxPolicy {
        self.policy
    }

    /// Executa um sidecar dentro da Gaiola de Silício (AppContainer/LPAC).
    ///
    /// Garante:
    /// 1. Perfil AppContainer criado dinamicamente (Drop limpa o Registro)
    /// 2. ACLs NTFS injetadas no diretório do projeto (Fail-Closed)
    /// 3. Handle DELETE_ON_CLOSE para diretório temporário efêmero
    /// 4. Loopback Exemption para IPC Zero-Copy
    /// 5. Spawn via CreateProcessW + STARTUPINFOEX (não bloqueia o Tokio)
    ///
    /// O `timeout_secs` é aplicado como teto absoluto do processo filho.
    #[cfg(target_os = "windows")]
    pub async fn execute_in_appcontainer(
        &self,
        command: &str,
        args: &[&str],
        timeout_secs: u64,
    ) -> Result<Vec<u8>, SandboxError> {
        self.execute_in_appcontainer_in_dir(command, args, timeout_secs, &self.repo_path).await
    }

    #[cfg(target_os = "windows")]
    pub async fn execute_in_appcontainer_in_dir(
        &self,
        command: &str,
        args: &[&str],
        timeout_secs: u64,
        execution_root: &Path,
    ) -> Result<Vec<u8>, SandboxError> {
        self.validate_execution_root(execution_root)?;
        let resolved = resolve_command(command, args, execution_root)?;
        self.enforce_host_path_policy(&resolved)?;

        // Gera nome único do perfil AppContainer baseado em timestamp para evitar
        // colisões entre execuções paralelas do mesmo sidecar.
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let container_name = format!("soda-sidecar-{ts}");

        info!(
            command,
            container_name = %container_name,
            repo_path = %self.repo_path.display(),
            cwd = %execution_root.display(),
            timeout_secs,
            "AppContainer: preparando Gaiola de Silicio"
        );

        // ── Passo 1: Cria o perfil AppContainer ──────────────────────────────
        // O Drop garante DeleteAppContainerProfile + FreeSid rigorosamente.
        let profile = create_appcontainer_profile(&container_name)?;

        // ── Passo 2: Diretório temporário efêmero ────────────────────────────
        // Criado no %TEMP% do host; o handle DELETE_ON_CLOSE o evaporará no Drop.
        let ephemeral_dir = std::env::temp_dir().join(format!("soda-ac-{ts}"));
        std::fs::create_dir_all(&ephemeral_dir).map_err(|e| SandboxError::AppContainerSetupFailed {
            detail: format!("Falha ao criar diretório efêmero '{}': {e}", ephemeral_dir.display()),
        })?;

        // ── Passo 3: Handle DELETE_ON_CLOSE (evaporação automática) ──────────
        let ephemeral_handle = SendHandle(open_dir_delete_on_close(&ephemeral_dir)?);

        // ── Passo 4: Muro do NTFS — Fail-Closed ─────────────────────────────
        // FILE_GENERIC_READ | FILE_GENERIC_EXECUTE = 0x0012_00A9
        // Concede leitura/execução no diretório do projeto.
        grant_ntfs_acl(&self.repo_path, profile.sid, 0x0012_00A9u32)
            .map_err(|e| {
                // Higiene: fecha o handle efêmero antes de propagar o erro.
                unsafe { CloseHandle(ephemeral_handle.0); }
                e
            })?;

        // FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE = 0x0012_01FFu32
        // Concede leitura/escrita/execução na pasta temporária.
        grant_ntfs_acl(&ephemeral_dir, profile.sid, 0x0012_01FFu32)
            .map_err(|e| {
                unsafe { CloseHandle(ephemeral_handle.0); }
                e
            })?;

        // ── Passo 5: Loopback Exemption (best-effort) ────────────────────────
        // Permite ao sidecar se conectar via loopback ao Named Pipe do Gateway.
        let loopback_ok = set_loopback_exemption(&container_name);
        if !loopback_ok {
            warn!(
                container_name = %container_name,
                "AppContainer: loopback exemption nao configurado; IPC via loopback pode falhar"
            );
        }

        // ── Passo 6: Spawn em AppContainer (spawn_blocking — anti-deadlock Tokio) ──
        // Toda lógica bloqueante de Win32 ocorre dentro de spawn_blocking.
        // Os callbacks de hidratação do ProjFS podem responder enquanto isso.
        // profile é movido para o spawn_blocking (unsafe impl Send) e dropado lá,
        // garantindo DeleteAppContainerProfile+FreeSid antes do retorno.
        let program = resolved.program.clone();
        let spawn_args = resolved.args.clone();
        let spawn_env = resolved.env.clone();
        let spawn_cwd = execution_root.to_path_buf();
        let timeout_ms = (timeout_secs.min(u32::MAX as u64) as u32)
            .saturating_mul(1000);

        let result = tokio::task::spawn_blocking(move || {
            // profile está no escopo do spawn_blocking — seu Drop ocorre aqui.
            let spawn_result = spawn_in_appcontainer_blocking(
                &program,
                &spawn_args,
                &spawn_env,
                &spawn_cwd,
                &profile,
                timeout_ms,
            );
            // Drop explícito: DeleteAppContainerProfile + FreeSid no thread bloqueante.
            drop(profile);
            spawn_result
        })
        .await
        .map_err(|e| SandboxError::ProcessSpawnFailed {
            reason: format!("spawn_blocking panicked: {e}"),
        })??;

        // ── Cleanup do handle efêmero ─────────────────────────────────────────
        // profile já foi dropado dentro do spawn_blocking acima.
        // ephemeral_handle.CloseHandle() → NTFS apaga o diretório automaticamente.
        unsafe { CloseHandle(ephemeral_handle.0); }

        let exit_code = result.exit_code;
        let stdout = result.stdout;
        let stderr_str = String::from_utf8_lossy(&result.stderr).trim().to_string();

        info!(
            command,
            container_name = %container_name,
            exit_code,
            stdout_bytes = stdout.len(),
            stderr_bytes = result.stderr.len(),
            "AppContainer: sidecar concluido"
        );

        if exit_code == 0 {
            Ok(stdout)
        } else {
            Err(SandboxError::ProcessNonZeroExit {
                exit_code,
                stderr: stderr_str,
                stdout,
            })
        }
    }

}

impl Drop for SandboxHandle {
    fn drop(&mut self) {
        let pids: Vec<u32> = {
            let guard = self.active_pids.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.iter().copied().collect()
        };

        if !pids.is_empty() {
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
        let cppcheck = timeout_profile("cppcheck", &["."], 30);
        assert_eq!(cppcheck.idle_timeout_secs, DEEP_FLOW_IDLE_TIMEOUT_SECS);
        assert_eq!(cppcheck.absolute_timeout_secs, None);

        let heavy = timeout_profile("opengrep", &["scan"], 30);
        assert_eq!(heavy.idle_timeout_secs, DEEP_FLOW_IDLE_TIMEOUT_SECS);
        assert_eq!(heavy.absolute_timeout_secs, None);

        let cargo_clippy = timeout_profile("cargo", &["clippy", "--message-format=json"], 30);
        assert_eq!(cargo_clippy.idle_timeout_secs, DEEP_FLOW_IDLE_TIMEOUT_SECS);
        assert_eq!(cargo_clippy.absolute_timeout_secs, None);

        let cargo_fetch = timeout_profile("cargo", &["fetch", "--manifest-path", "Cargo.toml"], 30);
        assert_eq!(cargo_fetch.idle_timeout_secs, DEEP_FLOW_IDLE_TIMEOUT_SECS);
        assert_eq!(cargo_fetch.absolute_timeout_secs, None);

        let cargo_metadata =
            timeout_profile("cargo", &["metadata", "--format-version", "1"], 30);
        assert_eq!(cargo_metadata.idle_timeout_secs, DEEP_FLOW_IDLE_TIMEOUT_SECS);
        assert_eq!(cargo_metadata.absolute_timeout_secs, None);
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

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let repo_dir = std::env::temp_dir().join(format!("soda-test-cwd-{ts}"));
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
        
        // Remove a pasta temporária de teste de forma limpa antes de asserir
        let _ = std::fs::remove_dir_all(&repo_dir);

        assert_eq!(output_str, expected);
    }
}
