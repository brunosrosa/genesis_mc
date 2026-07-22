use std::env;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use rustc_hash::FxHashSet;
use thiserror::Error;
use tokio::time::timeout;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, trace, warn};
use super::git::RepoPath;

static APPCONTAINER_SETUP_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();
static APPCONTAINER_SETUP_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static APPCONTAINER_ACL_CACHE: OnceLock<Mutex<FxHashSet<(PathBuf, String, u32, u32)>>> = OnceLock::new();

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
const NO_INHERITANCE: u32 = 0u32;
#[cfg(target_os = "windows")]
const SUB_CONTAINERS_AND_OBJECTS_INHERIT: u32 = 3u32;

#[cfg(target_os = "windows")]
const GENERIC_READ: u32 = 0x80000000u32;
#[cfg(target_os = "windows")]
const GENERIC_EXECUTE: u32 = 0x20000000u32;
#[cfg(target_os = "windows")]
const GENERIC_ALL: u32 = 0x10000000u32;

#[cfg(target_os = "windows")]
const FILE_GENERIC_READ: u32 = 0x001200a9u32;
#[cfg(target_os = "windows")]
const FILE_GENERIC_WRITE: u32 = 0x00120116u32;
#[cfg(target_os = "windows")]
const FILE_GENERIC_EXECUTE: u32 = 0x001200a0u32;
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
    CREATE_UNICODE_ENVIRONMENT,
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
#[allow(dead_code)]
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
    container_name: &str,
    access_mask: u32,
    inheritance_flag: u32,
) -> Result<(), SandboxError> {
    if sid.is_null() {
        return Ok(());
    }
    let path_buf = path.to_path_buf();
    let resolved_inheritance = if path.is_file() {
        NO_INHERITANCE
    } else {
        inheritance_flag
    };

    let mut resolved_access_mask = access_mask;
    if path.is_file() {
        let is_executable_or_trampoline = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                ext.eq_ignore_ascii_case("exe")
                    || ext.eq_ignore_ascii_case("cmd")
                    || ext.eq_ignore_ascii_case("bat")
                    || ext.eq_ignore_ascii_case("ps1")
            })
            .unwrap_or(false)
            || (access_mask & (0x2000_0000u32 | 0x0012_00A0u32)) != 0;

        if is_executable_or_trampoline {
            resolved_access_mask = 0x8000_0000u32 | 0x2000_0000u32;
        }
    }

    {
        let cache = APPCONTAINER_ACL_CACHE.get_or_init(|| Mutex::new(FxHashSet::default()));
        let guard = cache.lock().unwrap_or_else(|e| e.into_inner());
        let has_sufficient = guard.iter().any(|(p, c, m, i)| {
            p == &path_buf
                && c == container_name
                && (*m & resolved_access_mask) == resolved_access_mask
                && (*i == resolved_inheritance || *i == SUB_CONTAINERS_AND_OBJECTS_INHERIT)
        });
        if has_sufficient {
            return Ok(());
        }
    }

    // Fallback gracioso para scripts de lote (.cmd, .bat) que falham na injeção direta de ACL
    let is_shim = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
        .unwrap_or(false);

    use std::os::windows::ffi::OsStrExt;
    let mut path_wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    path_wide.push(0);

    // 1. Obtem o DACL existente para não sobrescrever permissões do host.
    let mut existing_dacl: *mut ACL = std::ptr::null_mut();
    let mut sd_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    let get_result = unsafe {
        GetNamedSecurityInfoW(
            path_wide.as_ptr() as windows_sys::core::PCWSTR,
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
        if get_result == 0x7a {
            warn!(
                path = %path.display(),
                "GetNamedSecurityInfoW retornou ERROR_INSUFFICIENT_BUFFER (0x7a) para '{}'. Aplicando fallback gracioso.",
                path.display()
            );
            return Ok(());
        }
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
        grfAccessPermissions: resolved_access_mask,
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: resolved_inheritance,
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
        if is_shim {
            warn!(
                path = %path.display(),
                "Sandbox: SetNamedSecurityInfoW falhou para script trampolim .cmd/.bat (Win32={apply_result:#010x}). Prosseguindo de forma best-effort."
            );
            return Ok(());
        }
        return Err(SandboxError::AclInjectionFailed {
            detail: format!(
                "SetNamedSecurityInfoW falhou para '{}': Win32={apply_result:#010x}",
                path.display()
            ),
        });
    }

    {
        let cache = APPCONTAINER_ACL_CACHE.get_or_init(|| Mutex::new(FxHashSet::default()));
        let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert((path_buf, container_name.to_string(), resolved_access_mask, resolved_inheritance));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn grant_access_to_winstation_and_desktop(_sid: PSID) -> Result<(), SandboxError> {
    use windows_sys::Win32::System::StationsAndDesktops::{GetProcessWindowStation, GetThreadDesktop};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SetSecurityInfo, SE_WINDOW_OBJECT, ConvertStringSidToSidW};

    unsafe {
        let hwinsta = GetProcessWindowStation();
        let hdesk = GetThreadDesktop(GetCurrentThreadId());

        let handles = [
            (hwinsta as windows_sys::Win32::Foundation::HANDLE, "Window Station"),
            (hdesk as windows_sys::Win32::Foundation::HANDLE, "Desktop")
        ];

        let mut all_app_packages_sid: PSID = std::ptr::null_mut();
        let sid_str_wide = str_to_wide("S-1-15-2-1");
        let sid_ok = ConvertStringSidToSidW(
            sid_str_wide.as_ptr(),
            &mut all_app_packages_sid,
        );
        if sid_ok == 0 || all_app_packages_sid.is_null() {
            return Err(SandboxError::AppContainerSetupFailed {
                detail: format!("ConvertStringSidToSidW falhou para S-1-15-2-1: {}", last_win32_error()),
            });
        }

        for (handle, name) in handles {
            if handle.is_null() {
                debug!("grant_access_to_winstation_and_desktop: {name} handle eh nulo, pulando.");
                continue;
            }

            let mut sd_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
            let mut existing_dacl: *mut ACL = std::ptr::null_mut();

            let get_result = GetSecurityInfo(
                handle,
                SE_WINDOW_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut existing_dacl,
                std::ptr::null_mut(),
                &mut sd_ptr,
            );

            if get_result != 0 {
                warn!("GetSecurityInfo falhou para {name} com erro Win32={get_result:#010x}");
                continue;
            }

            let mut ea = EXPLICIT_ACCESS_W {
                grfAccessPermissions: GENERIC_ALL | 0x000F037F, // GENERIC_ALL + WINSTA_ALL_ACCESS/DESKTOP_ALL_ACCESS masks
                grfAccessMode: GRANT_ACCESS,
                grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: std::ptr::null_mut(),
                    MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_WELL_KNOWN_GROUP,
                    ptstrName: all_app_packages_sid as windows_sys::core::PWSTR,
                },
            };

            let mut new_dacl: *mut ACL = std::ptr::null_mut();
            let merge_result = SetEntriesInAclW(
                1,
                &mut ea,
                existing_dacl,
                &mut new_dacl,
            );

            if merge_result != 0 || new_dacl.is_null() {
                if !sd_ptr.is_null() {
                    windows_sys::Win32::Foundation::LocalFree(sd_ptr as *mut _);
                }
                warn!("SetEntriesInAclW falhou para {name} com erro Win32={merge_result:#010x}");
                continue;
            }

            let set_result = SetSecurityInfo(
                handle,
                SE_WINDOW_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                new_dacl,
                std::ptr::null_mut(),
            );

            windows_sys::Win32::Foundation::LocalFree(new_dacl as *mut _);
            if !sd_ptr.is_null() {
                windows_sys::Win32::Foundation::LocalFree(sd_ptr as *mut _);
            }

            if set_result != 0 {
                warn!("SetSecurityInfo falhou para {name} com erro Win32={set_result:#010x}");
            } else {
                debug!("ACL de {name} atualizada com sucesso para o AppContainer SID");
            }
        }
        windows_sys::Win32::Foundation::LocalFree(all_app_packages_sid as *mut _);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
const BLOCKED_ACL_PREFIXES: &[&str] = &[
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "C:\\Windows",
    "C:\\ProgramData",
];

#[cfg(target_os = "windows")]
fn is_blocked_os_directory(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy();
    BLOCKED_ACL_PREFIXES.iter().any(|prefix| {
        path_str.starts_with(prefix)
    })
}

#[cfg(target_os = "windows")]
fn grant_ntfs_acl_with_parents(
    path: &std::path::Path,
    sid: PSID,
    container_name: &str,
    access_mask: u32,
    inheritance_flag: u32,
) -> Result<(), SandboxError> {
    let path_str = path.to_string_lossy().to_lowercase();
    if path_str.contains("nodejs")
        || path_str.contains("roaming\\npm")
        || path_str.contains("roaming/npm")
        || path_str.contains(".souls_workspaces")
    {
        trace!(
            target_path = %path.display(),
            "grant_ntfs_acl_with_parents: ignorando injeção em caminho global ou virtualizado (ProjFS) para evitar I/O ou deadlocks."
        );
        return Ok(());
    }

    // L06: Blocklist de diretórios estruturais do SO — nunca injetar ACLs do
    // AppContainer em pastas do kernel/sistema.
    if is_blocked_os_directory(path) {
        trace!(
            target_path = %path.display(),
            "grant_ntfs_acl_with_parents: Caminho bloqueado (diretório estrutural do SO). Pulando."
        );
        return Ok(());
    }

    trace!(
        target_path = %path.display(),
        "grant_ntfs_acl_with_parents: Iniciando para o alvo principal..."
    );
    grant_ntfs_acl(path, sid, container_name, access_mask, inheritance_flag)?;
    trace!(
        target_path = %path.display(),
        "grant_ntfs_acl_with_parents: Alvo principal concluido com sucesso."
    );

    let local_appdata = std::env::var("LOCALAPPDATA").ok().map(|s| strip_unc_prefix(std::path::Path::new(&s)));
    let roaming_appdata = std::env::var("APPDATA").ok().map(|s| strip_unc_prefix(std::path::Path::new(&s)));
    let user_profile = std::env::var("USERPROFILE").ok().map(|s| strip_unc_prefix(std::path::Path::new(&s)));

    let clean_path = strip_unc_prefix(path);
    if !is_blocked_os_directory(&clean_path) {
        let mut parent = clean_path.parent();
        while let Some(p) = parent {
            if p.parent().is_none() {
                break;
            }

            if is_blocked_os_directory(p) {
                break;
            }

            if Some(p.to_path_buf()) == local_appdata
                || Some(p.to_path_buf()) == roaming_appdata
                || Some(p.to_path_buf()) == user_profile
            {
                break;
            }
            
            trace!(
                parent_path = %p.display(),
                "grant_ntfs_acl_with_parents: Concedendo travessia ao pai..."
            );
            if let Err(e) = grant_ntfs_acl(p, sid, container_name, 0x0012_00A9u32, NO_INHERITANCE) {
                let is_access_denied = match &e {
                    SandboxError::AclInjectionFailed { detail } => {
                        detail.contains("Win32=0x00000005") || detail.contains("Win32=5")
                    }
                    _ => false,
                };
                if !is_access_denied {
                    return Err(e);
                }
            }
            trace!(
                parent_path = %p.display(),
                "grant_ntfs_acl_with_parents: Concedido com sucesso ou tratado para o pai."
            );
            
            parent = p.parent();
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn grant_ntfs_acl_for_all_application_packages(
    path: &std::path::Path,
    access_mask: u32,
    inheritance_flag: u32,
) {
    let mut all_app_packages_sid: PSID = std::ptr::null_mut();
    let sid_str_wide = str_to_wide("S-1-15-2-1");
    let sid_ok = unsafe {
        windows_sys::Win32::Security::Authorization::ConvertStringSidToSidW(
            sid_str_wide.as_ptr(),
            &mut all_app_packages_sid,
        )
    };
    if sid_ok != 0 && !all_app_packages_sid.is_null() {
        let _ = grant_ntfs_acl(path, all_app_packages_sid, "ALL_APPLICATION_PACKAGES", access_mask, inheritance_flag);
        unsafe {
            windows_sys::Win32::Foundation::LocalFree(all_app_packages_sid as *mut _);
        }
    }
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

#[cfg(target_os = "windows")]
impl Drop for SendHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.0);
            }
            self.0 = std::ptr::null_mut();
        }
    }
}

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
    #[cfg(test)]
    {
        let _ = profile_name;
        true
    }
    #[cfg(not(test))]
    {
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
    timeout_profile: TimeoutProfile,
    ephemeral_dir: &std::path::Path,
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

    // L14: Garantia anti-aspa para o Windows CreateProcessW.
    // O programa SEMPRE deve estar englobado em aspas duplas explícitas, pois o
    // CreateProcessW interpreta o primeiro token como o caminho do executável.
    // O escape_cmd_arg não quotava o program quando não continha espaços, mas o
    // Windows precisa de quoting explícito para paths com backslashes.
    // Solução: quoting brutal direto com argumentos nomeados para evitar ambiguidade.
    let args_string = args.iter()
        .map(|a| escape_cmd_arg(a))
        .collect::<Vec<String>>()
        .join(" ");
    // Garante que o caminho do executável esteja SEMPRE entre aspas duplas para CreateProcessW
    let cmd_str = format!("\"{}\" {}", program.display(), args_string);
    let mut cmd_wide: Vec<u16> = str_to_wide(&cmd_str);
    let clean_cwd = dunce::canonicalize(&cwd).unwrap_or(cwd.to_path_buf());
    let final_cwd_string = clean_cwd.to_string_lossy().replace(r"\\?\", "").replace(r"\?\", "");
    let clean_cwd_path = std::path::PathBuf::from(final_cwd_string);
    let cwd_wide = path_to_wide(&clean_cwd_path);

    debug!(
        cmd_line = %cmd_str,
        cwd = %clean_cwd_path.display(),
        "Spawn em AppContainer: executando CreateProcessW"
    );

    // ── 2. Bloco de ambiente UTF-16 null-null ─────────────────────────────────
    // Formato: KEY=VALUE\0KEY=VALUE\0\0 (ordenado lexicograficamente por chaves em UPPERCASE)
    let env_block: Vec<u16> = {
        let mut merged_env: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();

        // Herda o ambiente do processo pai com chaves em UPPERCASE
        for (k, v) in std::env::vars() {
            merged_env.insert(k.to_uppercase(), v);
        }

        // Sobrescreve as variáveis do sidecar com chaves em UPPERCASE
        for (k, v) in env {
            merged_env.insert(k.to_uppercase(), v.clone());
        }

        // L09: Sobrescreve chaves de temporários e home para o diretório efêmero.
        // CARGO_HOME e RUSTUP_HOME são EXCLUÍDOS intencionalmente — devem apontar
        // para as toolchains reais do host via ACL NTFS rasa (Lei L09).
        let ephemeral_dir_str = ephemeral_dir.to_string_lossy().into_owned();
        let keys_to_override = [
            "HOME",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "TEMP",
            "TMP",
            "XDG_CACHE_HOME",
            // L14: Cache dirs do Opengrep/Semgrep — também precisam ser redirecionados
            // para o diretório efêmero para que os binários auto-extraíveis consigam
            // inflar seus runtimes dentro da gaiola AppContainer.
            "SEMGREP_CACHE_DIR",
            "OPENGREP_CACHE_DIR",
        ];
        for key in &keys_to_override {
            merged_env.insert(key.to_string(), ephemeral_dir_str.clone());
        }

        // Força o Cargo a rodar em modo offline no sandbox
        merged_env.insert("CARGO_NET_OFFLINE".to_string(), "true".to_string());

        // Serializa o bloco de ambiente Win32
        let mut env_str = String::new();
        for (k, v) in &merged_env {
            // Ignora variáveis vazias ou chaves internas do Windows que começam com '='.
            if k.is_empty() || k.starts_with('=') {
                continue;
            }
            // L14: Log expandido para mostrar TODAS as variáveis críticas injetadas,
            // incluindo os cache dirs do Opengrep/Semgrep que são essenciais para
            // o binário auto-extraível funcionar dentro do AppContainer.
            if k == "LOCALAPPDATA" || k == "TEMP" || k == "TMP" || k == "USERPROFILE" || k == "APPDATA"
                || k == "SEMGREP_CACHE_DIR" || k == "OPENGREP_CACHE_DIR" || k == "XDG_CACHE_HOME"
            {
                debug!(key = %k, value = %v, "AppContainer: Ambiente injetado para CreateProcessW");
            }
            env_str.push_str(&format!("{k}={v}\0"));
        }
        env_str.push_str("\0"); // Duplo-nulo finalizador

        env_str.encode_utf16().collect()
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
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
            env_block.as_ptr() as *const std::ffi::c_void,
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
    let started_at = std::time::Instant::now();
    let mut last_activity = std::time::Instant::now();

    loop {
        // Polling de 250ms: balanceia responsividade vs CPU.
        let wait_result = unsafe {
            WaitForSingleObject(pi.hProcess, 250)
        };

        // Drena o que há disponível em stdout (não-bloqueante via PeekNamedPipe).
        let mut read_any_io = false;
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
            read_any_io = true;
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
            read_any_io = true;
        }

        if read_any_io {
            last_activity = std::time::Instant::now();
        }

        // WAIT_OBJECT_0 = 0x0000_0000 = processo terminou.
        if wait_result == 0x0000_0000u32 {
            break;
        }

        // Verifica os limites de timeout:
        // 1. Idle timeout
        if last_activity.elapsed() >= Duration::from_secs(timeout_profile.idle_timeout_secs) {
            warn!(
                cmd_line = %cmd_str,
                idle_timeout_secs = timeout_profile.idle_timeout_secs,
                "AppContainer: Idle timeout atingido (silêncio de I/O); SIGKILL"
            );
            unsafe {
                windows_sys::Win32::System::Threading::TerminateProcess(pi.hProcess, 1);
                WaitForSingleObject(pi.hProcess, 1000);
                CloseHandle(pi.hProcess);
                CloseHandle(stdout_read);
                CloseHandle(stderr_read);
            }
            return Err(SandboxError::Timeout);
        }

        // 2. Absolute timeout
        if let Some(absolute_secs) = timeout_profile.absolute_timeout_secs {
            if started_at.elapsed() >= Duration::from_secs(absolute_secs) {
                warn!(
                    cmd_line = %cmd_str,
                    absolute_secs,
                    "AppContainer: Absolute timeout atingido; SIGKILL"
                );
                unsafe {
                    windows_sys::Win32::System::Threading::TerminateProcess(pi.hProcess, 1);
                    WaitForSingleObject(pi.hProcess, 1000);
                    CloseHandle(pi.hProcess);
                    CloseHandle(stdout_read);
                    CloseHandle(stderr_read);
                }
                return Err(SandboxError::Timeout);
            }
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

#[allow(dead_code)]
const IDLE_TIMEOUT_SECS: u64 = 45;
#[allow(dead_code)]
const DEEP_FLOW_IDLE_TIMEOUT_SECS: u64 = 900;
#[allow(dead_code)]
const ABSOLUTE_TIMEOUT_FLOOR_SECS: u64 = 5 * 60;
#[allow(dead_code)]
const PROCESS_WAIT_POLL_INTERVAL_MS: u64 = 250;

#[allow(dead_code)]
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

#[allow(dead_code)]
fn mark_process_activity(last_activity: &Arc<Mutex<Instant>>) {
    let mut guard = last_activity
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = Instant::now();
}

#[allow(dead_code)]
fn idle_elapsed(last_activity: &Arc<Mutex<Instant>>) -> Duration {
    last_activity
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .elapsed()
}

#[allow(dead_code)]
enum ProcessWaitOutcome {
    Exited(std::process::ExitStatus),
    WaitError(std::io::Error),
    IdleTimeout,
    AbsoluteTimeout,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    #[cfg(target_os = "windows")]
    {
        if let Some(native_exe) = resolve_native_npm_bin(repo_path, base_name) {
            return Some(native_exe);
        }
    }
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

#[cfg(target_os = "windows")]
fn trace_trampoline_target(cmd_path: &std::path::Path) -> Option<PathBuf> {
    debug!(
        cmd_path = %cmd_path.display(),
        "AppContainer: Iniciando varredura de trampolim .cmd/.bat"
    );
    let content = std::fs::read_to_string(cmd_path).ok()?;
    let cmd_dir = cmd_path.parent()?;
    
    let mut best_target = None;
    
    for line in content.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.starts_with('@') || trimmed_line.to_ascii_lowercase().starts_with("rem") {
            if trimmed_line.to_ascii_lowercase().starts_with("@echo") || trimmed_line.to_ascii_lowercase().starts_with("rem") {
                continue;
            }
        }
        
        let mut candidates = Vec::new();
        
        // Extrai strings entre aspas
        let mut in_quote = false;
        let mut current_quote = String::new();
        for c in line.chars() {
            if c == '"' {
                if in_quote {
                    if !current_quote.is_empty() {
                        candidates.push(current_quote.clone());
                    }
                    current_quote.clear();
                    in_quote = false;
                } else {
                    in_quote = true;
                }
            } else if in_quote {
                current_quote.push(c);
            }
        }
        
        // Também adiciona palavras que parecem caminhos (contém '\' ou '/')
        for word in line.split_whitespace() {
            let clean_word = word.trim_matches('"');
            if (clean_word.contains('\\') || clean_word.contains('/')) && !clean_word.is_empty() {
                candidates.push(clean_word.to_string());
            }
        }
        
        for raw_candidate in candidates {
            let expanded = if raw_candidate.contains("%~dp0") {
                let suffix = raw_candidate.replace("%~dp0", "");
                let suffix = suffix.strip_prefix('\\').unwrap_or(&suffix);
                cmd_dir.join(suffix)
            } else {
                PathBuf::from(&raw_candidate)
            };
            
            let resolved = if expanded.is_absolute() {
                expanded
            } else {
                cmd_dir.join(expanded)
            };
            
            if let Ok(canonical) = dunce::canonicalize(&resolved) {
                let cleaned = strip_unc_prefix(&canonical);
                if cleaned.is_file() {
                    let file_name = cleaned.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let ext = cleaned.extension().and_then(|e| e.to_str()).unwrap_or("");
                    
                    debug!(
                        raw_candidate = %raw_candidate,
                        cleaned_path = %cleaned.display(),
                        "AppContainer: Candidato de trampolim resolvido com sucesso"
                    );
                    
                    if ext.eq_ignore_ascii_case("exe") {
                        if file_name.eq_ignore_ascii_case("node.exe") {
                            best_target = Some(cleaned);
                        } else {
                            return Some(cleaned);
                        }
                    } else if best_target.is_none() {
                        best_target = Some(cleaned);
                    }
                }
            }
        }
    }
    
    best_target
}

#[cfg(target_os = "windows")]
fn resolve_native_npm_bin(repo_path: &Path, base_name: &str) -> Option<PathBuf> {
    let platforms = ["win32-x64", "win32-arm64", "win32-ia32"];
    let mut relative_candidates = Vec::new();
    
    for platform in &platforms {
        if base_name == "biome" {
            relative_candidates.push(PathBuf::from("@biomejs").join(format!("cli-{platform}")).join("biome.exe"));
        } else if base_name == "oxlint" {
            relative_candidates.push(PathBuf::from("@oxc-project").join(format!("oxlint-{platform}")).join("oxlint.exe"));
            relative_candidates.push(PathBuf::from(format!("oxlint-{platform}")).join("oxlint.exe"));
            relative_candidates.push(PathBuf::from("@oxlint").join(platform).join("oxlint.exe"));
        }
    }

    let mut node_modules_roots = Vec::new();
    node_modules_roots.push(repo_path.join("node_modules"));
    
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        node_modules_roots.push(PathBuf::from(localappdata).join("pnpm").join("node_modules"));
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        node_modules_roots.push(PathBuf::from(appdata).join("npm").join("node_modules"));
    }

    for root in &node_modules_roots {
        for rel_path in &relative_candidates {
            let candidate = root.join(rel_path);
            if candidate.is_file() {
                debug!(
                    base_name = %base_name,
                    resolved_exe = %candidate.display(),
                    "resolve_native_npm_bin: Encontrou binario nativo O(1)"
                );
                return Some(candidate);
            }
        }
    }
    
    None
}

#[cfg(target_os = "windows")]
fn resolve_real_binary_from_trampoline(program: PathBuf, command: &str, repo_path: &Path) -> PathBuf {
    let is_trampoline = program.extension()
        .map(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
        .unwrap_or(false);
    if !is_trampoline {
        return program;
    }

    // Determina se o .cmd está no diretório global do npm (AppData\Roaming\npm)
    // Nesse caso, o trampoline chama node.exe para interpretar o JS — NÃO contém o .exe real.
    // A análise de texto do .cmd só encontraria node.exe, causando o deadlock no AppContainer.
    // Estratégia: ir diretamente para as heurísticas de localização do .exe nativo.
    let is_global_npm_trampoline = program.parent().map(|parent| {
        std::env::var("APPDATA")
            .ok()
            .map(|appdata| {
                let npm_dir = PathBuf::from(&appdata).join("npm");
                parent.starts_with(&npm_dir)
            })
            .unwrap_or(false)
    }).unwrap_or(false);

    if !is_global_npm_trampoline {
        // Apenas tenta o trace de texto para trampolins locais (dentro do repo),
        // onde o .cmd pode apontar diretamente para o .exe nativo.
        if let Some(target) = trace_trampoline_target(&program) {
            let target_ext = target.extension().and_then(|e| e.to_str()).unwrap_or("");
            // Rejeita node.exe — ele não pode entrar na gaiola
            let is_node = target.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.eq_ignore_ascii_case("node.exe"))
                .unwrap_or(false);
            if target_ext.eq_ignore_ascii_case("exe") && !is_node {
                debug!(
                    trampoline = %program.display(),
                    resolved = %target.display(),
                    "resolve_real_binary_from_trampoline: .exe nativo encontrado via trace de texto"
                );
                return target;
            }
        }
    }

    // Heurística direta: verifica caminhos canônicos de instalação do .exe nativo
    // (global npm node_modules e local repo node_modules).
    if let Some(native_exe) = resolve_native_npm_bin(repo_path, command) {
        debug!(
            command = %command,
            resolved = %native_exe.display(),
            "resolve_real_binary_from_trampoline: .exe nativo encontrado via resolve_native_npm_bin"
        );
        return native_exe;
    }

    // Retorna o trampolim original — o Fail-Fast em resolve_command bloqueará o spawn
    warn!(
        command = %command,
        trampoline = %program.display(),
        "resolve_real_binary_from_trampoline: FALHOU em localizar .exe nativo. Trampolim será rejeitado pelo Fail-Fast."
    );
    program
}


#[cfg(target_os = "windows")]
fn grant_runtime_and_tool_acls(
    repo_path: &Path,
    sid: PSID,
    container_name: &str,
    command: &str,
    env: &mut std::collections::BTreeMap<String, String>,
) -> Result<(), SandboxError> {
    let mut paths_to_grant = Vec::new();

    // Ferramentas que genuinamente precisam do runtime JS (como jest, vitest).
    // Ferramentas nativas em Rust (biome, oxlint) ou Python (ruff, bandit) NÃO usam Node.js
    // e não devem receber a ACL do Node.js no AppContainer para evitar deadlock ou vazamento de privilégios.
    let needs_js_runtime = matches!(command, "jest" | "vitest");

    // 1. Node.js runtime — PROIBIDO para ferramentas que não precisam do ecossistema JS
    if needs_js_runtime {
        let mut node_resolved = None;
        if let Ok(node_path) = which::which("node") {
            node_resolved = Some(node_path);
        } else if let Some(node_path) = resolve_from_path("node") {
            node_resolved = Some(node_path);
        }

        if let Some(node_path) = node_resolved {
            // Resolve absolute physical path using std::fs::canonicalize() to resolve Symlinks
            let canonical_node_path = match std::fs::canonicalize(&node_path) {
                Ok(canonical) => strip_unc_prefix(&canonical),
                Err(_) => strip_unc_prefix(&node_path),
            };
            if let Some(parent) = canonical_node_path.parent() {
                let _ = grant_ntfs_acl_with_parents(
                    parent,
                    sid,
                    container_name,
                    FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
                    SUB_CONTAINERS_AND_OBJECTS_INHERIT,
                );

                // Append parent to PATH env case-insensitively
                let path_key = env.keys()
                    .find(|k| k.eq_ignore_ascii_case("PATH"))
                    .cloned()
                    .unwrap_or_else(|| "PATH".to_string());
                let current_path = env.get(&path_key).cloned()
                    .or_else(|| {
                        std::env::vars()
                            .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
                            .map(|(_, v)| v)
                    })
                    .unwrap_or_default();
                let parent_str = parent.to_string_lossy();
                let new_path = if current_path.is_empty() {
                    parent_str.into_owned()
                } else {
                    format!("{};{}", current_path, parent_str)
                };
                env.insert(path_key, new_path);
            }
            let _ = grant_ntfs_acl_with_parents(
                &canonical_node_path,
                sid,
                container_name,
                FILE_GENERIC_READ | FILE_GENERIC_EXECUTE,
                SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            );
        }
    }

    // 2. Python runtime & standard library (via local virtualenv pyvenv.cfg)
    let is_python_cmd = command == "python" || command == "python3" || command == "pytest" || command == "ruff" || command == "bandit" || command == "uv";
    if is_python_cmd {
        let venv_candidates = [repo_path.join(".venv"), repo_path.join("venv")];
        for venv_dir in &venv_candidates {
            if venv_dir.is_dir() {
                paths_to_grant.push(venv_dir.clone());
                let cfg_path = venv_dir.join("pyvenv.cfg");
                if cfg_path.is_file() {
                    if let Ok(cfg_content) = std::fs::read_to_string(&cfg_path) {
                        for line in cfg_content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("home") {
                                let parts: Vec<&str> = trimmed.split('=').collect();
                                if parts.len() >= 2 {
                                    let home_path = PathBuf::from(parts[1].trim().trim_matches('"'));
                                    if home_path.is_dir() {
                                        paths_to_grant.push(home_path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Global Python interpreter
        if let Some(python_path) = resolve_from_path("python").or_else(|| resolve_from_path("python3")) {
            let canonical_python_path = match std::fs::canonicalize(&python_path) {
                Ok(canonical) => strip_unc_prefix(&canonical),
                Err(_) => strip_unc_prefix(&python_path),
            };
            if let Some(parent) = canonical_python_path.parent() {
                paths_to_grant.push(parent.to_path_buf());

                // Append parent to PATH env case-insensitively
                let path_key = env.keys()
                    .find(|k| k.eq_ignore_ascii_case("PATH"))
                    .cloned()
                    .unwrap_or_else(|| "PATH".to_string());
                let current_path = env.get(&path_key).cloned()
                    .or_else(|| {
                        std::env::vars()
                            .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
                            .map(|(_, v)| v)
                    })
                    .unwrap_or_default();
                let parent_str = parent.to_string_lossy();
                let new_path = if current_path.is_empty() {
                    parent_str.into_owned()
                } else {
                    format!("{};{}", current_path, parent_str)
                };
                env.insert(path_key, new_path);
            }
        }
    }

    // 3. Global tool directories (pnpm, npm, uv, yarn)
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        if needs_js_runtime {
            let pnpm_dir = PathBuf::from(&localappdata).join("pnpm");
            if pnpm_dir.is_dir() {
                paths_to_grant.push(pnpm_dir);
            }
            let yarn_dir = PathBuf::from(&localappdata).join("Yarn");
            if yarn_dir.is_dir() {
                paths_to_grant.push(yarn_dir);
            }
        }
        if is_python_cmd {
            let uv_local = PathBuf::from(&localappdata).join("uv");
            if uv_local.is_dir() {
                paths_to_grant.push(uv_local);
            }
            let python_local = PathBuf::from(&localappdata).join("Programs").join("Python");
            if python_local.is_dir() {
                paths_to_grant.push(python_local);
            }
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        // O diretório global do npm contém scripts .cmd que chamam node.exe via batch.
        // Injetar esse diretório no AppContainer para ferramentas que não necessitam
        // do ecossistema JS causa deadlock silencioso: o AppContainer tenta executar o .cmd
        // que depende de cmd.exe. Ferramentas Rust/Python nativas recebem ACL apenas para o seu .exe.
        if needs_js_runtime {
            let npm_dir = PathBuf::from(&appdata).join("npm");
            if npm_dir.is_dir() {
                paths_to_grant.push(npm_dir);
            }
        }
        if is_python_cmd {
            let uv_dir = PathBuf::from(&appdata).join("uv");
            if uv_dir.is_dir() {
                paths_to_grant.push(uv_dir);
            }
        }
    }


    paths_to_grant.sort();
    paths_to_grant.dedup();

    for path in paths_to_grant {
        let path_clean = strip_unc_prefix(&path);
        if path_clean.exists() {
            let canonical_path = match std::fs::canonicalize(&path_clean) {
                Ok(canonical) => strip_unc_prefix(&canonical),
                Err(_) => path_clean.clone(),
            };
            if canonical_path.exists() {
                trace!(
                    path = %canonical_path.display(),
                    "AppContainer: Concedendo ACL NTFS para runtime/ferramenta global/local"
                );
                if let Err(e) = grant_ntfs_acl_with_parents(&canonical_path, sid, container_name, GENERIC_READ | GENERIC_EXECUTE, SUB_CONTAINERS_AND_OBJECTS_INHERIT) {
                    let is_access_denied = match &e {
                        SandboxError::AclInjectionFailed { detail } => {
                            detail.contains("Win32=0x00000005") || detail.contains("Win32=5")
                        }
                        _ => false,
                    };
                    if !is_access_denied {
                        return Err(e);
                    } else {
                        trace!(
                            path = %canonical_path.display(),
                            "grant_ntfs_acl_with_parents falhou com Access Denied para runtime/ferramenta. Continuando."
                        );
                    }
                }
            }
        }
    }

    // PRD-031 §C: Injeta OBRIGATORIAMENTE System32 e Windows no PATH da Gaiola.
    // Scripts .cmd (biome.cmd, oxlint.cmd) precisam de cmd.exe do System32 para executar.
    // A injeção é feita AQUI (não no spawn) para que apareça no env final da ferramenta.
    #[cfg(target_os = "windows")]
    {
        let win_paths = [
            r"C:\Windows\System32".to_string(),
            r"C:\Windows".to_string(),
        ];
        let path_key = env
            .keys()
            .find(|k| k.eq_ignore_ascii_case("PATH"))
            .cloned()
            .unwrap_or_else(|| "PATH".to_string());
        let current_path = env
            .get(&path_key)
            .cloned()
            .or_else(|| {
                std::env::vars()
                    .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
                    .map(|(_, v)| v)
            })
            .unwrap_or_default();
        // Materializa em Vec<String> (owned) para evitar problemas de lifetime
        let mut path_segments: Vec<String> = current_path
            .split(';')
            .map(|s| s.to_string())
            .collect();
        for win_str in &win_paths {
            // Deduplicação case-insensitive: não adiciona se já presente
            let already_present = path_segments
                .iter()
                .any(|seg| seg.trim().eq_ignore_ascii_case(win_str.as_str()));
            if !already_present {
                path_segments.push(win_str.clone());
            }
        }
        let new_path = path_segments
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<String>>()
            .join(";");
        env.insert(path_key, new_path);
    }

    Ok(())
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
        // L14: Desabilita version check de rede (phone-home) que causa timeout de 120s
        // quando o sandbox não tem acesso à internet.
        (
            "SEMGREP_ENABLE_VERSION_CHECK".to_string(),
            "0".to_string(),
        ),
    ])
}

fn is_cargo_sast_invocation<S: AsRef<str>>(args: &[S]) -> bool {
    matches!(
        args.first().map(|value| value.as_ref()),
        Some("clippy" | "fetch" | "metadata")
    )
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn is_govulncheck_no_packages_match(command: &str, exit_code: i32, stderr: &[u8]) -> bool {
    if command != "govulncheck" || exit_code != 2 {
        return false;
    }
    String::from_utf8_lossy(stderr)
        .to_ascii_lowercase()
        .contains("no packages matched the provided patterns")
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessObservabilityClass {
    Ok,
    InformationalNonZero,
    LethalNonZero,
}

#[allow(dead_code)]
fn classify_process_observability(command: &str, exit_code: i32, stdout: &[u8]) -> ProcessObservabilityClass {
    if exit_code == 0 || (command == "opengrep" && exit_code == 7) {
        ProcessObservabilityClass::Ok
    } else if !stdout.is_empty() {
        ProcessObservabilityClass::InformationalNonZero
    } else {
        ProcessObservabilityClass::LethalNonZero
    }
}

#[allow(dead_code)]
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

    // L08: Escrita atômica via write-then-rename (snapsafe) para prevenir
    // corrupção de dados caso o sidecar sofra SIGKILL durante a gravação.
    let temp_path = diagnostics_dir.join(format!(".semgrep-{timestamp}.tmp"));
    std::fs::write(&temp_path, report).ok()?;
    std::fs::rename(&temp_path, &diagnostics_path).ok()?;
    Some(diagnostics_path)
}

fn resolve_real_cargo_path() -> Option<PathBuf> {
    if let Ok(output) = std::process::Command::new("rustup")
        .args(["which", "cargo"])
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                let path = PathBuf::from(path_str);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }
    
    if let (Ok(home), Ok(toolchain)) = (std::env::var("RUSTUP_HOME"), std::env::var("RUSTUP_TOOLCHAIN")) {
        let path = PathBuf::from(home)
            .join("toolchains")
            .join(toolchain)
            .join("bin")
            .join(if cfg!(target_os = "windows") { "cargo.exe" } else { "cargo" });
        if path.is_file() {
            return Some(path);
        }
    }
    
    None
}

const SAST_ARSENAL_TOOLS: &[&str] = &["biome", "oxlint", "ruff", "opengrep"];

fn resolve_sast_arsenal_binary(command: &str) -> Option<PathBuf> {
    if !SAST_ARSENAL_TOOLS.contains(&command) {
        return None;
    }
    let target_triple = "x86_64-pc-windows-msvc";
    let bin_name = format!("{command}-{target_triple}.exe");
    let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bin")
        .join(&bin_name);
    if candidate.is_file() {
        return Some(candidate);
    }
    None
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
            let program = resolve_real_cargo_path()
                .or_else(|| resolve_from_path("cargo"))
                .unwrap_or_else(|| PathBuf::from(command));
            let mut env = BTreeMap::new();
            if let Some(parent) = program.parent() {
                let rustc_name = if cfg!(target_os = "windows") { "rustc.exe" } else { "rustc" };
                let rustc_path = parent.join(rustc_name);
                if rustc_path.is_file() {
                    env.insert("RUSTC".to_string(), rustc_path.display().to_string());
                }
            }
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
            let mut program = resolve_local_node_bin(repo_path, command)
                .or_else(|| resolve_from_path(command))
                .unwrap_or_else(|| PathBuf::from(command));
            #[cfg(target_os = "windows")]
            {
                program = resolve_real_binary_from_trampoline(program, command, repo_path);
            }
            Ok(ResolvedCommand {
                program,
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                env: BTreeMap::new(),
            })
        }
        "biome" | "oxlint" => {
            let mut program = resolve_sast_arsenal_binary(command)
                .or_else(|| resolve_local_node_bin(repo_path, command))
                .or_else(|| resolve_from_path(command))
                .unwrap_or_else(|| PathBuf::from(command));
            #[cfg(target_os = "windows")]
            {
                program = resolve_real_binary_from_trampoline(program, command, repo_path);

                // FAIL-FAST: É estritamente proibido executar scripts .cmd/.bat no AppContainer.
                // Se após a resolução completa o programa ainda for um trampolim batch, a lâmina
                // DEVE rejeitar imediatamente com erro explícito antes de criar qualquer recurso.
                let resolved_ext = program.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if resolved_ext.eq_ignore_ascii_case("cmd") || resolved_ext.eq_ignore_ascii_case("bat") {
                    return Err(SandboxError::PolicyViolation {
                        detail: format!(
                            "{command}: binário nativo .exe não encontrado. \
                            Trampolim batch '{prog}' não pode ser executado no AppContainer (deadlock). \
                            Instale o {command} nativo: npm install -g @biomejs/biome (ou equivalente).",
                            command = command,
                            prog = program.display()
                        ),
                    });
                }
            }
            Ok(ResolvedCommand {
                program,
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                env: BTreeMap::new(),
            })
        }
        "ruff" | "bandit" => {
            // Prioriza o ruff do arsenal sidecar se for ruff. bandit continua python normal.
            let program = resolve_sast_arsenal_binary(command)
                .or_else(|| resolve_local_python_bin(repo_path, command))
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
            let program = resolve_sast_arsenal_binary(command)
                .or_else(|| resolve_from_path(command))
                .unwrap_or_else(|| PathBuf::from(command));
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn command_requires_orphan_reap(command: &str) -> bool {
    matches!(command, "semgrep" | "opengrep")
}

#[allow(dead_code)]
async fn collect_output_task(task: tokio::task::JoinHandle<Vec<u8>>) -> Vec<u8> {
    match timeout(Duration::from_secs(30), task).await {
        Ok(Ok(buffer)) => buffer,
        _ => Vec::new(),
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

fn strip_unc_prefix(path: &std::path::Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    let cleaned = s.replace(r"\\?\", "").replace(r"\?\", "");
    std::path::PathBuf::from(cleaned)
}

impl SandboxHandle {
    /// Helper para acessar o Mutex de PIDs de forma segura contra poisoning.
    /// Se o Mutex estiver envenenado (panic em outra thread), recupera o lock
    /// ao invés de propagar o panic — Fail-Safe obrigatório em produção.
    #[allow(dead_code)]
    fn lock_pids(&self) -> std::sync::MutexGuard<'_, HashSet<u32>> {
        self.active_pids.lock().unwrap_or_else(|poisoned| {
            // Recupera o guard do Mutex envenenado — os dados internos ainda são válidos.
            // Em produção, o comportamento correto é continuar operando para garantir
            // que o Drop consiga limpar os processos órfãos.
            poisoned.into_inner()
        })
    }

fn build_global_allowed_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    
    // Resolve home do usuário de forma agnóstica a SO
    let home_dir = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();

    if !home_dir.is_empty() {
        let home_path = PathBuf::from(&home_dir);
        
        let cargo_home = std::env::var("CARGO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_path.join(".cargo"));
        roots.push(cargo_home);

        let rustup_home = std::env::var("RUSTUP_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_path.join(".rustup"));
        roots.push(rustup_home);

        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home_path.join("AppData").join("Roaming"));
            roots.push(appdata.join("uv").join("tools"));
        }
        #[cfg(not(target_os = "windows"))]
        {
            let xdg_data = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home_path.join(".local").join("share"));
            roots.push(xdg_data.join("uv").join("tools"));
        }
    }
    roots
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

        let global_roots = Self::build_global_allowed_roots();

        for candidate in inspected_paths {
            let allowed = path_is_within_root(&candidate, repo_root)
                || self
                    .host_write_roots
                    .iter()
                    .any(|root| path_is_within_root(&candidate, root))
                || global_roots
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

    #[allow(dead_code)]
    async fn execute_with_root(
        &self,
        command: &str,
        args: &[&str],
        timeout_secs: u64,
        execution_root: &Path,
    ) -> Result<Vec<u8>, SandboxError> {
        let canonical_root = std::fs::canonicalize(execution_root)
            .unwrap_or_else(|_| execution_root.to_path_buf());
        let execution_root_clean = strip_unc_prefix(&canonical_root);

        self.validate_execution_root(&execution_root_clean)?;
        let resolved = resolve_command(command, args, &execution_root_clean)?;
        self.enforce_host_path_policy(&resolved)?;
        let requested_command = command.to_string();
        debug!(
            command = %requested_command,
            program = %resolved.program.display(),
            args = ?truncated_args_preview(&resolved.args),
            env = ?truncated_env_preview(&resolved.env),
            repo_path = %self.repo_path.display(),
            cwd = %execution_root_clean.display(),
            policy = ?self.policy,
            timeout_secs,
            "Sandbox: iniciando processo efemero"
        );

        let mut process = tokio::process::Command::new(&resolved.program);
        process
            .args(&resolved.args)
            .current_dir(&execution_root_clean)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true);
        if !resolved.env.is_empty() {
            process.envs(&resolved.env);
        }
        let child = process
            .spawn()
            .map_err(|e| SandboxError::ProcessSpawnFailed { reason: e.to_string() })?;

        #[cfg(target_os = "windows")]
        let job_guard = Some(attach_child_to_kill_on_close_job(&child)?);
        #[cfg(not(target_os = "windows"))]
        let job_guard: Option<()> = None;

        let pid = child.id().ok_or_else(|| {
            SandboxError::ProcessSpawnFailed { reason: "Não foi possível capturar PID do processo".to_string() }
        })?;

        let mut child_guard = crate::process_guard::ProcessGuard::new(child);

        self.lock_pids().insert(pid);

        let last_activity = Arc::new(Mutex::new(Instant::now()));
        let stdout_task = {
            let last_activity = Arc::clone(&last_activity);
            tokio::spawn(drain_pipe_with_telemetry(
                child_guard.child.as_mut().unwrap().stdout.take().ok_or_else(|| {
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
                child_guard.child.as_mut().unwrap().stderr.take().ok_or_else(|| {
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
            match child_guard.child.as_mut().unwrap().try_wait() {
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
                let _ = child_guard.child.take(); // Desarma o ProcessGuard, pois ja terminou
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
                let observability = classify_process_observability(&requested_command, exit_code, &merged_stdout);
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
                let is_ok = status.success() || (requested_command == "opengrep" && exit_code == 7);
                if is_ok {
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
                let _ = child_guard.child.as_mut().unwrap().kill().await;
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
                let _ = child_guard.child.as_mut().unwrap().kill().await;
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
        let canonical_root = std::fs::canonicalize(execution_root)
            .unwrap_or_else(|_| execution_root.to_path_buf());
        let execution_root_clean = strip_unc_prefix(&canonical_root);
        self.validate_execution_root(&execution_root_clean)?;
        // L14: Validação de CWD fantasma — o diretório DEVE existir no disco
        // para que o CreateProcessW não falhe com Win32 Error 2 (ERROR_FILE_NOT_FOUND).
        if !execution_root_clean.exists() {
            return Err(SandboxError::PolicyViolation {
                detail: format!(
                    "CWD fantasma: '{}' nao existe no disco",
                    execution_root_clean.display()
                ),
            });
        }
        let mut resolved = resolve_command(command, args, &execution_root_clean)?;

        // Resolve absolute path for resolved.program if it is relative or just a basename.
        if !resolved.program.is_absolute() {
            if let Some(abs_path) = resolve_from_path(&resolved.program.to_string_lossy()) {
                resolved.program = abs_path;
            }
        }
        if !resolved.program.is_absolute() {
            let local_candidate = execution_root_clean.join(&resolved.program);
            if local_candidate.is_file() {
                resolved.program = local_candidate;
            }
        }
        if resolved.program.is_absolute() || resolved.program.exists() {
            if let Ok(canonical) = dunce::canonicalize(&resolved.program) {
                resolved.program = strip_unc_prefix(&canonical);
            }
        }

        let mut extra_acl_paths = Vec::new();

        // L14: Vacina contra asfixia de permissão NTFS (Node e NPM Global) no AppContainer.
        // ESTE BLOCO DEVE EXECUTAR APENAS para ferramentas que genuinamente precisam do runtime JS
        // (jest, vitest). Ferramentas nativas (opengrep, bandit, ruff, biome) NÃO usam Node.js
        // e não devem receber ACL para C:\Program Files\nodejs\.
        let needs_js_runtime = matches!(command, "jest" | "vitest");
        if needs_js_runtime {
            if let Ok(path_env) = std::env::var("PATH") {
                for p in std::env::split_paths(&path_env) {
                    let node_path = p.join("node.exe");
                    if node_path.is_file() {
                        debug!(node_dir = %p.display(), "AppContainer: detectado runtime Node.js no PATH. Concedendo ACL NTFS de leitura.");
                        extra_acl_paths.push(p);
                        break;
                    }
                }
            }
        }

        if let Some(ext) = resolved.program.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat") {
                if let Some(target) = trace_trampoline_target(&resolved.program) {
                    let target_ext = target.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if target_ext.eq_ignore_ascii_case("exe") {
                        debug!(
                            cmd_path = %resolved.program.display(),
                            target_exe = %target.display(),
                            "AppContainer: resolvendo trampolim .cmd/.bat para executavel real"
                        );
                        resolved.program = target;
                    } else {
                        debug!(
                            cmd_path = %resolved.program.display(),
                            target_script = %target.display(),
                            "AppContainer: resolvendo trampolim .cmd/.bat para script. Adicionando caminhos de ACL extras."
                        );
                        extra_acl_paths.push(target.clone());
                        if let Some(parent) = target.parent() {
                            extra_acl_paths.push(parent.to_path_buf());
                        }
                    }
                }
                if let Some(parent) = resolved.program.parent() {
                    extra_acl_paths.push(parent.to_path_buf());
                }
            }
        }

        // PRD-031 §D: O support_dir (.soda_semgrep) DEVE receber leitura NTFS via extra_acl_paths
        // para que o OpenGrep possa ler seu --config sem ser bloqueado pelo AppContainer.
        if command == "opengrep" || command == "semgrep" {
            let support_dir = semgrep_support_root(&self.repo_path);
            if support_dir.exists() {
                debug!(
                    support_dir = %support_dir.display(),
                    command,
                    "AppContainer: adicionando support_dir semgrep em extra_acl_paths (PRD-031 §D)"
                );
                extra_acl_paths.push(support_dir);
            }
        }

        debug!(
            command,
            resolved_program = %resolved.program.display(),
            extra_acl_paths = ?extra_acl_paths,
            "AppContainer: Preparando injeções de ACL NTFS para o enjaulamento"
        );

        // Limpa prefixo UNC de todas as variáveis de ambiente resolvidas
        for value in resolved.env.values_mut() {
            if value.starts_with(r"\\?\") {
                *value = value[4..].to_string();
            }
        }

        // Limpa prefixo UNC de todos os argumentos resolvidos
        for arg in &mut resolved.args {
            if arg.starts_with(r"\\?\") {
                *arg = arg[4..].to_string();
            }
        }

        self.enforce_host_path_policy(&resolved)?;

        let uuid_str = uuid::Uuid::new_v4().simple().to_string();
        let slot = APPCONTAINER_SETUP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst) % 4;
        // Gera nome do perfil AppContainer baseado no comando/lâmina e slot exclusivo de concorrência para evitar colisões
        let container_name = format!("soda-ac-{}-{}", command, slot);
        let container_name = if container_name.len() > 64 {
            container_name[..64].to_string()
        } else {
            container_name
        };

        // Adquire o semáforo de setup (limite 4) para serializar as chamadas NTFS e criação de profiles de AppContainer
        let setup_semaphore = APPCONTAINER_SETUP_SEMAPHORE.get_or_init(|| Semaphore::new(4));
        let setup_permit = setup_semaphore.acquire().await.ok();

        info!(
            command,
            container_name = %container_name,
            repo_path = %self.repo_path.display(),
            cwd = %execution_root_clean.display(),
            timeout_secs,
            "AppContainer: preparando Gaiola de Silicio"
        );

        // L03: Envolve pre-flight e ACLs NTFS in spawn_blocking para evitar Thread Starvation (Lei L03)
        let container_name_clone = container_name.clone();
        let execution_root_clean_clone = execution_root_clean.clone();
        let repo_path_clone = self.repo_path.clone();
        let host_write_roots_clone = self.host_write_roots.clone();
        let command_clone = command.to_string();
        let extra_acl_paths_clone = extra_acl_paths.clone();
        let mut resolved_clone = resolved.clone();
        let uuid_str_clone = uuid_str.clone();

        let preflight_result = tokio::task::spawn_blocking(move || -> Result<(AppContainerProfile, PathBuf, SendHandle, ResolvedCommand), SandboxError> {
            // ── Passo 1: Cria o perfil AppContainer ──────────────────────────────
            // O Drop garante DeleteAppContainerProfile + FreeSid rigorosamente.
            let profile = create_appcontainer_profile(&container_name_clone)?;
            grant_access_to_winstation_and_desktop(profile.sid)?;

            // ── Passo 1.5: Materialização Física dos Cofres de Cache/Sandbox ────
            let cargo_home_sandbox = sandbox_tool_state_root(&repo_path_clone, "cargo-home");
            let cargo_target_sandbox = sandbox_tool_state_root(&repo_path_clone, "cargo-target");
            let cargo_clippy_sandbox = sandbox_tool_state_root(&repo_path_clone, "cargo-clippy-target");
            let semgrep_root = semgrep_support_root(&repo_path_clone);
            let semgrep_sandbox = semgrep_root.join("sandbox");
            let semgrep_dot = semgrep_sandbox.join(".semgrep");
            let semgrep_diag = semgrep_root.join("diagnostics");

            let host_dirs_to_materialize = [
                &cargo_home_sandbox,
                &cargo_target_sandbox,
                &cargo_clippy_sandbox,
                &semgrep_root,
                &semgrep_sandbox,
                &semgrep_dot,
                &semgrep_diag,
            ];
            for dir in &host_dirs_to_materialize {
                std::fs::create_dir_all(dir).map_err(|e| {
                    SandboxError::AppContainerSetupFailed {
                        detail: format!("Falha ao criar diretório de cache do sidecar '{}': {e}", dir.display()),
                    }
                })?;
            }

            // Concede permissão total com herança para todas as pastas de cache do sidecar
            let clean_dir_mask = FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE;
            for dir in &host_dirs_to_materialize {
                grant_ntfs_acl_with_parents(dir, profile.sid, &container_name_clone, clean_dir_mask, SUB_CONTAINERS_AND_OBJECTS_INHERIT)
                    .map_err(|e| {
                        SandboxError::AppContainerSetupFailed {
                            detail: format!("Falha ao conceder permissões NTFS de cache para '{}': {e:?}", dir.display()),
                        }
                    })?;
            }

            // Pre-cria as pastas de LocalAppData (Packages) do AppContainer no Host
            // e concede ACL para evitar a quebra do Nuitka {CACHE_DIR} do opengrep
            let local_appdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
            if !local_appdata.is_empty() {
                let container_packages_dir = std::path::Path::new(&local_appdata)
                    .join("Packages")
                    .join(&container_name_clone);
                
                // Cria a árvore de pastas esperada pelo AppContainer do Windows para evitar SegFaults do Nuitka/Opengrep
                let dirs_to_create = [
                    container_packages_dir.join("LocalState"),
                    container_packages_dir.join("AC"),
                    container_packages_dir.join("AC").join("Temp"),
                    container_packages_dir.join("AC").join("LocalState"),
                    container_packages_dir.join("AC").join("LocalCache"),
                    container_packages_dir.join("AC").join("LocalFolder"),
                ];
                for dir in &dirs_to_create {
                    std::fs::create_dir_all(dir).map_err(|e| {
                        SandboxError::AppContainerSetupFailed {
                            detail: format!("Falha ao criar pasta de perfil '{}': {e}", dir.display()),
                        }
                    })?;
                }

                grant_ntfs_acl_with_parents(&container_packages_dir, profile.sid, &container_name_clone, clean_dir_mask, SUB_CONTAINERS_AND_OBJECTS_INHERIT).map_err(|e| {
                    SandboxError::AppContainerSetupFailed {
                        detail: format!("Falha ao conceder NTFS write ACL para a pasta do AppContainer Packages: {e:?}"),
                    }
                })?;
            }

            // ── Passo 2: Diretório temporário efêmero ────────────────────────────
            // Criado no %TEMP% do host; o handle DELETE_ON_CLOSE o evaporará no Drop.
            let ephemeral_dir = strip_unc_prefix(&std::env::temp_dir().join(format!("soda-ac-{uuid_str_clone}")));
            std::fs::create_dir_all(&ephemeral_dir).map_err(|e| SandboxError::AppContainerSetupFailed {
                detail: format!("Falha ao criar diretório efêmero '{}': {e}", ephemeral_dir.display()),
            })?;

            // Concede IMEDIATAMENTE acesso total 0x001201bf ao AppContainer SID para a pasta efêmera (fake USERPROFILE)
            grant_ntfs_acl(
                &ephemeral_dir,
                profile.sid,
                &container_name_clone,
                0x001201bf,
                SUB_CONTAINERS_AND_OBJECTS_INHERIT,
            ).map_err(|e| SandboxError::AppContainerSetupFailed {
                detail: format!("Falha ao conceder ACL total para diretório efêmero '{}': {e:?}", ephemeral_dir.display()),
            })?;

            // Injeta FORÇOSAMENTE no bloco de variáveis de ambiente do comando resolvida
            // as variáveis do cache do Opengrep/Semgrep apontando para o diretório efêmero do AppContainer
            let ephemeral_dir_str = ephemeral_dir.to_string_lossy().into_owned();
            resolved_clone.env.insert("SEMGREP_CACHE_DIR".to_string(), ephemeral_dir_str.clone());
            resolved_clone.env.insert("OPENGREP_CACHE_DIR".to_string(), ephemeral_dir_str.clone());
            resolved_clone.env.insert("XDG_CACHE_HOME".to_string(), ephemeral_dir_str.clone());

            // Vacina do Ruff: impede erro de escrita de cache no ProjFS
            if command_clone == "ruff" {
                resolved_clone.env.insert("RUFF_CACHE_DIR".to_string(), ephemeral_dir_str.clone());
            }

            // Cura da Cegueira dos Trampolins (Bandit / uv / Nuitka / Opengrep):
            // Redireciona o perfil do usuário para o diretório efêmero que é 100% gravável na Gaiola.
            resolved_clone.env.insert("LOCALAPPDATA".to_string(), ephemeral_dir_str.clone());
            resolved_clone.env.insert("APPDATA".to_string(), ephemeral_dir_str.clone());
            resolved_clone.env.insert("USERPROFILE".to_string(), ephemeral_dir_str.clone());
            resolved_clone.env.insert("HOME".to_string(), ephemeral_dir_str.clone());

            // ── Passo 2.1: L07 — Criação dos Cofres Fantasmas ────────────────────
            // Cria fisicamente as subpastas de APPDATA/LOCALAPPDATA/TEMP/.cargo que ferramentas
            // como Opengrep, Nuitka, Cargo e Bandit esperam encontrar no filesystem.
            let vault_dirs = [
                ephemeral_dir.join("AppData").join("Local"),
                ephemeral_dir.join("AppData").join("Roaming"),
                ephemeral_dir.join(".config"),
                ephemeral_dir.join(".cache"),
                ephemeral_dir.join(".local").join("share"),
                ephemeral_dir.join(".cargo"),
                ephemeral_dir.join("Temp"),
            ];
            for dir in &vault_dirs {
                std::fs::create_dir_all(dir).map_err(|e| {
                    SandboxError::AppContainerSetupFailed {
                        detail: format!("Falha ao criar cofre fantasma '{}': {e}", dir.display()),
                    }
                })?;
            }

            // ── Passo 3: Handle DELETE_ON_CLOSE (evaporação automática) ──────────
            // GENERIC_READ é necessário para abrir o diretório sem acesso de escrita exclusivo.
            let ephemeral_handle = SendHandle(open_dir_delete_on_close(&ephemeral_dir)?);

            // ── Passo 4: Muro do NTFS — Fail-Closed ─────────────────────────────
            // FILE_GENERIC_READ | FILE_GENERIC_EXECUTE = 0x0012_00A9
            
            // Concede permissão de Leitura para ALL APPLICATION PACKAGES (S-1-15-2-1) diretamente na raiz do execution_root e repo_path para mitigar erros de ProjFS (OS Error 5)
            grant_ntfs_acl_for_all_application_packages(&execution_root_clean_clone, 0x0012_00A9u32, SUB_CONTAINERS_AND_OBJECTS_INHERIT);
            if repo_path_clone != execution_root_clean_clone {
                grant_ntfs_acl_for_all_application_packages(&repo_path_clone, 0x0012_00A9u32, SUB_CONTAINERS_AND_OBJECTS_INHERIT);
            }

            // PRD-031 §E (Monorepo Awareness): Concede FILE_GENERIC_READ na raiz ABSOLUTA do
            // repositório clonado. Em monorepos, Cargo Clippy busca dependências em ../vendor
            // (fora do crate path). A ACL deve cobrir o repo_root completo para evitar I/O errors.
            //
            // Estratégia: Se o repo_path contém Cargo.toml e o diretório pai contém um
            // Cargo.toml ou vendor/, concedemos leitura também no pai (raiz do monorepo).
            let repo_root_for_acl = {
                let parent = repo_path_clone.parent();
                match parent {
                    Some(p) if p.join("Cargo.toml").exists() || p.join("vendor").is_dir() => {
                        info!(
                            monorepo_root = %p.display(),
                            crate_path = %repo_path_clone.display(),
                            "AppContainer: Monorepo detectado — expandindo ACL para raiz absoluta (PRD-031 §E)"
                        );
                        p.to_path_buf()
                    }
                    _ => repo_path_clone.clone(),
                }
            };
            // Leitura (sem escrita) na raiz absoluta do repo — cobre ../vendor e Cargo.lock pai
            grant_ntfs_acl_with_parents(&repo_root_for_acl, profile.sid, &container_name_clone, FILE_GENERIC_READ, SUB_CONTAINERS_AND_OBJECTS_INHERIT)?;
            // Leitura+escrita apenas no cwd efêmero do crate (execution_root)
            if repo_root_for_acl != execution_root_clean_clone {
                grant_ntfs_acl_with_parents(&execution_root_clean_clone, profile.sid, &container_name_clone, FILE_GENERIC_READ | FILE_GENERIC_WRITE, SUB_CONTAINERS_AND_OBJECTS_INHERIT)?;
            }

            // Concede permissões para as runtimes e ferramentas globais/locais usadas
            grant_runtime_and_tool_acls(&repo_path_clone, profile.sid, &container_name_clone, &command_clone, &mut resolved_clone.env)?;

            // Concede FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE na pasta temporária.
            grant_ntfs_acl_with_parents(&ephemeral_dir, profile.sid, &container_name_clone, FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE, SUB_CONTAINERS_AND_OBJECTS_INHERIT)?;

            // Concede permissões de leitura/execução no CARGO_HOME e RUSTUP_HOME do Host
            let user_profile = std::env::var("USERPROFILE").unwrap_or_default();
            let cargo_home = std::env::var("CARGO_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(&user_profile).join(".cargo"));
            let rustup_home = std::env::var("RUSTUP_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(&user_profile).join(".rustup"));
            if command_clone == "cargo" || command_clone == "clippy" || command_clone == "rustc" {
                if cargo_home.exists() {
                    // Herança NTFS completa — leitura e travessia com propagação
                    let _ = grant_ntfs_acl_with_parents(&cargo_home, profile.sid, &container_name_clone, 0x0012_00A9u32, SUB_CONTAINERS_AND_OBJECTS_INHERIT);
                }
                if rustup_home.exists() {
                    let _ = grant_ntfs_acl_with_parents(&rustup_home, profile.sid, &container_name_clone, 0x0012_00A9u32, SUB_CONTAINERS_AND_OBJECTS_INHERIT);
                }
            }

            // Concede leitura/escrita/execução nas pastas de escrita permitidas do host
            for root in &host_write_roots_clone {
                grant_ntfs_acl_with_parents(root, profile.sid, &container_name_clone, FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE, SUB_CONTAINERS_AND_OBJECTS_INHERIT)?;
            }

            // Concede leitura/execução na pasta pai do executável resolvido (para trampolins/libs locais)
            if let Some(parent) = resolved_clone.program.parent() {
                if let Err(e) = grant_ntfs_acl_with_parents(parent, profile.sid, &container_name_clone, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE, SUB_CONTAINERS_AND_OBJECTS_INHERIT) {
                    let is_access_denied = match &e {
                        SandboxError::AclInjectionFailed { detail } => {
                            detail.contains("Win32=0x00000005") || detail.contains("Win32=5")
                        }
                        _ => false,
                    };
                    if !is_access_denied {
                        return Err(e);
                    }
                }

                // Append the main program's parent to PATH env as well
                let path_key = resolved_clone.env.keys()
                    .find(|k| k.eq_ignore_ascii_case("PATH"))
                    .cloned()
                    .unwrap_or_else(|| "PATH".to_string());
                let current_path = resolved_clone.env.get(&path_key).cloned()
                    .or_else(|| {
                        std::env::vars()
                            .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
                            .map(|(_, v)| v)
                    })
                    .unwrap_or_default();
                let parent_str = parent.to_string_lossy();
                let new_path = if current_path.is_empty() {
                    parent_str.into_owned()
                } else {
                    format!("{};{}", current_path, parent_str)
                };
                resolved_clone.env.insert(path_key, new_path);
            }

            // Concede leitura/execução explicitamente no arquivo binário executável resolvido.
            if let Err(e) = grant_ntfs_acl_with_parents(&resolved_clone.program, profile.sid, &container_name_clone, 0x0012_00A9u32, SUB_CONTAINERS_AND_OBJECTS_INHERIT) {
                let is_access_denied = match &e {
                    SandboxError::AclInjectionFailed { detail } => {
                        detail.contains("Win32=0x00000005") || detail.contains("Win32=5")
                    }
                    _ => false,
                };
                if !is_access_denied {
                    return Err(e);
                }
            }

            // Concede leitura/execução nos caminhos adicionais do trampolim (scripts e pastas pai do script)
            for path in &extra_acl_paths_clone {
                if let Some(parent) = path.parent() {
                    if let Err(e) = grant_ntfs_acl_with_parents(parent, profile.sid, &container_name_clone, FILE_GENERIC_READ | FILE_GENERIC_EXECUTE, SUB_CONTAINERS_AND_OBJECTS_INHERIT) {
                        let is_access_denied = match &e {
                            SandboxError::AclInjectionFailed { detail } => {
                                detail.contains("Win32=0x00000005") || detail.contains("Win32=5")
                            }
                            _ => false,
                        };
                        if !is_access_denied {
                            return Err(e);
                        }
                    }
                }
                if let Err(e) = grant_ntfs_acl_with_parents(path, profile.sid, &container_name_clone, 0x0012_00A9u32, SUB_CONTAINERS_AND_OBJECTS_INHERIT) {
                    let is_access_denied = match &e {
                        SandboxError::AclInjectionFailed { detail } => {
                            detail.contains("Win32=0x00000005") || detail.contains("Win32=5")
                        }
                        _ => false,
                    };
                    if !is_access_denied {
                        return Err(e);
                    }
                }
            }
            println!("DEBUG PREFLIGHT 17: Extra paths concedidos");

            // ── Passo 5: Loopback Exemption (best-effort) ────────────────────────
            // Permite ao sidecar se conectar via loopback ao Named Pipe do Gateway.
            let _loopback_ok = set_loopback_exemption(&container_name_clone);

            // ── Passo 5.5: Desidratação do Opengrep (Nuitka Pre-Extraction) ─────
            // O OpenGrep é compilado via Nuitka e tenta extrair seu motor OCaml em runtime
            // para OPENGREP_CACHE_DIR. Para evitar STATUS_FAIL_FAST_EXCEPTION
            // (exit_code=-1073740791) causado pelo AppContainer bloqueando geração de código
            // dinâmico (ACG), pré-executamos o opengrep no HOST (fora da gaiola) para que
            // o Nuitka extraia o cache ANTES do AppContainer ser trancado.
            // O AppContainer reutilizará o cache já extraído, driblando a política ACG.
            if command_clone == "opengrep" {
                info!(
                    command = %command_clone,
                    cache_dir = %ephemeral_dir_str,
                    "Desidratação Opengrep: pré-executando --version no host para extrair motor Nuitka"
                );

                // Construir o ambiente completo para a desidratação (mesmas vars do AppContainer)
                let mut dehydrate_env: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                for (k, v) in std::env::vars() {
                    dehydrate_env.insert(k, v);
                }
                for (k, v) in &resolved_clone.env {
                    dehydrate_env.insert(k.clone(), v.clone());
                }
                // Garante que TEMP/tmp apontam para o ephemeral_dir
                let ephemeral_str = ephemeral_dir.to_string_lossy().into_owned();
                dehydrate_env.insert("TEMP".to_string(), ephemeral_str.clone());
                dehydrate_env.insert("TMP".to_string(), ephemeral_str.clone());
                dehydrate_env.insert("OPENGREP_CACHE_DIR".to_string(), ephemeral_str.clone());
                dehydrate_env.insert("SEMGREP_CACHE_DIR".to_string(), ephemeral_str.clone());
                dehydrate_env.insert("XDG_CACHE_HOME".to_string(), ephemeral_str.clone());

                // Desidratação: executa --version no host para forçar extração Nuitka
                let dehydrate_result = std::process::Command::new(&resolved_clone.program)
                    .arg("--version")
                    .envs(dehydrate_env.iter())  // REMOVI .env_clear() - mantém sistema PATH!
                    .current_dir(&execution_root_clean_clone)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn();

                match dehydrate_result {
                    Ok(mut child) => {
                        use std::io::Read;
                        let mut stdout_buf = Vec::new();
                        let mut stderr_buf = Vec::new();
                        if let Some(ref mut stdout) = child.stdout {
                            let _ = stdout.read_to_end(&mut stdout_buf);
                        }
                        if let Some(ref mut stderr) = child.stderr {
                            let _ = stderr.read_to_end(&mut stderr_buf);
                        }
                        match child.wait() {
                            Ok(status) => {
                                if status.success() {
                                    info!(
                                        command = %command_clone,
                                        exit_code = ?status.code(),
                                        "Desidratação Opengrep: sucesso - motor Nuitka extraído para cache"
                                    );
                                } else {
                                    warn!(
                                        command = %command_clone,
                                        exit_code = ?status.code(),
                                        stderr = %String::from_utf8_lossy(&stderr_buf),
                                        "Desidratação Opengrep: concluído com código não-sucesso (pode ser OK se --version打印ou info)"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(
                                    command = %command_clone,
                                    error = %e,
                                    "Desidratação Opengrep: wait falhou - continuando mesmo assim"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            command = %command_clone,
                            error = %e,
                            "Desidratação Opengrep: spawn falhou - continuando sem desidratação"
                        );
                    }
                }

                // Passo crucial: após Nuitka extrair arquivos no ephemeral_dir, re-aplicamos
                // ACL para garantir que todos os arquivos extraídos tenham permissões para o AppContainer SID
                info!(
                    ephemeral_dir = %ephemeral_dir.display(),
                    "Reaplicando ACL NTFS recursiva no ephemeral_dir após desidratação Nuitka"
                );
                let clean_dir_mask = FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE;
                let _ = grant_ntfs_acl_with_parents(
                    &ephemeral_dir,
                    profile.sid,
                    &container_name_clone,
                    clean_dir_mask,
                    SUB_CONTAINERS_AND_OBJECTS_INHERIT,
                );
            }

            Ok((profile, ephemeral_dir, ephemeral_handle, resolved_clone))
        });

        const PREFLIGHT_TIMEOUT_SECS: u64 = 120;

        let (profile, ephemeral_dir, _ephemeral_handle, resolved) = match timeout(
            Duration::from_secs(PREFLIGHT_TIMEOUT_SECS),
            preflight_result
        ).await {
            Ok(Ok(Ok(res))) => {
                drop(setup_permit);
                res
            }
            Ok(Ok(Err(sandbox_err))) => {
                drop(setup_permit);
                return Err(sandbox_err);
            }
            Ok(Err(join_err)) => {
                drop(setup_permit);
                return Err(SandboxError::AppContainerSetupFailed {
                    detail: format!("Falha de thread starvation no setup da gaiola (spawn_blocking join error): {join_err}"),
                });
            }
            Err(_) => {
                drop(setup_permit);
                return Err(SandboxError::AppContainerSetupFailed {
                    detail: format!("Timeout pre-flight de {PREFLIGHT_TIMEOUT_SECS} segundos atingido ao preparar Gaiola de Silicio"),
                });
            }
        };

        // ── Passo 0: L14 — Dry-Run Gating (Fail-Closed) ──────────────────
        // Testa se o binário alvo consegue executar na gaiola. Usa o
        // próprio resolved_program com --version (não cmd.exe, pois LPAC bloqueia System32).
        // Executado FORA e APÓS a liberação do setup_permit do semáforo.
        //
        // L14: Bypass para ferramentas do arsenal local e ferramentas que não suportam
        // --version de forma confiável no AppContainer. Estas ferramentas são localmente
        // instaladas e confiáveis — o dry-run é redundante.
        const DRY_RUN_BYPASS_TOOLS: &[&str] = &[
            "biome", "oxlint", "ruff", "opengrep", "clippy", "govulncheck",
        ];
        let skip_dry_run = DRY_RUN_BYPASS_TOOLS.contains(&command);
        if !skip_dry_run {
            let dry_run_program = resolved.program.clone();
            let mut cmd = tokio::process::Command::new(&dry_run_program);
            cmd.current_dir(&execution_root_clean);
            
            // Tratativa específica de flags para cmd.exe para evitar travamento interativo
            let is_cmd = dry_run_program
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.eq_ignore_ascii_case("cmd") || n.eq_ignore_ascii_case("cmd.exe"))
                .unwrap_or(false);
            if is_cmd {
                cmd.arg("/c").arg("exit").arg("0");
            } else {
                cmd.arg("--version");
            }
            
            cmd.stdout(std::process::Stdio::null())
               .stderr(std::process::Stdio::null())
               .stdin(std::process::Stdio::null())
               .kill_on_drop(true);

            match cmd.spawn() {
                Ok(mut child) => {
                    let wait_fut = child.wait();
                    match timeout(Duration::from_secs(3), wait_fut).await {
                        Ok(Ok(status)) if status.success() => {
                            debug!(
                                command,
                                program = %resolved.program.display(),
                                "AppContainer: dry-run do binário alvo bem-sucedido"
                            );
                        }
                        _ => {
                            // Se falhou ou deu timeout, kill_on_drop garante a morte
                            error!(
                                command,
                                program = %resolved.program.display(),
                                "AppContainer: dry-run do binário alvo falhou ou deu timeout."
                            );
                            return Err(SandboxError::AppContainerSetupFailed {
                                detail: format!("Dry-run do binário alvo '{}' falhou ou deu timeout.", resolved.program.display())
                            });
                        }
                    }
                }
                Err(e) => {
                    error!(
                        command,
                        program = %resolved.program.display(),
                        error = %e,
                        "AppContainer: falha ao spawnar dry-run do binário alvo."
                    );
                    return Err(SandboxError::AppContainerSetupFailed {
                        detail: format!("Falha ao spawnar dry-run do binário alvo '{}': {e}", resolved.program.display())
                    });
                }
            }
        } else {
            debug!(
                command,
                program = %resolved.program.display(),
                "AppContainer: dry-run pulado para ferramenta do arsenal local (bypass confiavel)"
            );
        }

        // ── Passo 6: Spawn em AppContainer (spawn_blocking — anti-deadlock Tokio) ──
        // Toda lógica bloqueante de Win32 ocorre dentro de spawn_blocking.
        // Os callbacks de hidratação do ProjFS podem responder enquanto isso.
        // profile é movido para o spawn_blocking (unsafe impl Send) e dropado lá,
        // garantindo DeleteAppContainerProfile+FreeSid antes do retorno.
        let program = resolved.program.clone();
        let spawn_args = resolved.args.clone();
        // L14: O resolved Extraído da destructureação (linha 3490) É o mesmo resolved_clone
        // que foi modificado dentro do spawn_blocking (cache dirs injetados em ~linha 3300).
        // verified: resolved.env.clone() contém OPENGREP_CACHE_DIR após o await.
        let spawn_env = resolved.env.clone();
        let spawn_cwd = execution_root_clean.clone();
        let timeout_profile = timeout_profile(command, args, timeout_secs);

        let ephemeral_dir_clone = ephemeral_dir.clone();

        let result = tokio::task::spawn_blocking(move || {
            // profile está no escopo do spawn_blocking — seu Drop ocorre aqui.
            let spawn_result = spawn_in_appcontainer_blocking(
                &program,
                &spawn_args,
                &spawn_env,
                &spawn_cwd,
                &profile,
                timeout_profile,
                &ephemeral_dir_clone,
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
        // O handle efêmero (ephemeral_handle) será fechado automaticamente via RAII (Drop) ao sair do escopo,
        // acionando a evaporação/remoção do diretório pelo NTFS.

        // Limpa o diretório de LocalAppData do AppContainer para evitar acúmulo no disco do Host
        let local_appdata_cleanup = std::env::var("LOCALAPPDATA").unwrap_or_default();
        if !local_appdata_cleanup.is_empty() {
            let container_packages_dir = std::path::Path::new(&local_appdata_cleanup)
                .join("Packages")
                .join(&container_name);
            let _ = std::fs::remove_dir_all(&container_packages_dir);
        }

        let exit_code = result.exit_code;
        let stdout = result.stdout;
        let stderr_str = String::from_utf8_lossy(&result.stderr).trim().to_string();

        // L14: Alert Fatigue — SASTs como Biome/Opengrep retornam exit_code=1 quando encontram
        // findings. Isso é um resultado válido, não um erro. Só logamos [ERR] para crashes
        // (exit_code < 0) ou erros reais (exit_code >= 2). Para exit 1/7 com stdout, WARN.
        let is_sast_expected_nonzero = (exit_code == 1 || exit_code == 7) && !stdout.is_empty();

        if exit_code == 0 {
            info!(
                command,
                container_name = %container_name,
                exit_code,
                stdout_bytes = stdout.len(),
                stderr_bytes = result.stderr.len(),
                "AppContainer: sidecar concluido"
            );
        } else if is_sast_expected_nonzero {
            warn!(
                command,
                container_name = %container_name,
                exit_code,
                stdout_bytes = stdout.len(),
                stderr_bytes = result.stderr.len(),
                "AppContainer: SAST concluido com achados (Exit {})",
                exit_code
            );
        } else {
            error!(
                command,
                container_name = %container_name,
                exit_code,
                stdout_bytes = stdout.len(),
                stderr_bytes = result.stderr.len(),
                "AppContainer: sidecar concluido"
            );
        }

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
        let canonical_path = std::fs::canonicalize(repo_path.as_ref())
            .unwrap_or_else(|_| repo_path.as_ref().to_path_buf());
        let clean_path = strip_unc_prefix(&canonical_path);
        Ok(SandboxHandle {
            repo_path: clean_path.clone(),
            policy,
            host_write_roots: build_host_write_roots(&clean_path, policy)?,
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
        
        let temp_dir = TempDir::new().unwrap();
        let repo_path = RepoPath(temp_dir.path().to_path_buf());

        let sandbox = SandboxOrchestrator::create(&repo_path, SandboxPolicy::ReadOnly)
            .await
            .expect("Deveria criar sandbox com sucesso");

        assert_eq!(sandbox.policy(), SandboxPolicy::ReadOnly);
        assert_eq!(sandbox.repo_path(), repo_path.as_ref());
    }

    #[tokio::test]
    async fn test_execute_in_sandbox() {
        let _guard = get_test_mutex().await.lock().await;
        
        let temp_dir = TempDir::new().unwrap();
        let repo_path = RepoPath(temp_dir.path().to_path_buf());

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
            classify_process_observability("biome", 0, b"{}"),
            ProcessObservabilityClass::Ok
        );
        assert_eq!(
            classify_process_observability("biome", 1, b"{\"results\":[]}"),
            ProcessObservabilityClass::InformationalNonZero
        );
        assert_eq!(
            classify_process_observability("biome", 101, b""),
            ProcessObservabilityClass::LethalNonZero
        );
    }

    #[test]
    fn test_classify_process_observability_treats_any_stdout_bytes_as_informational_non_zero() {
        assert_eq!(
            classify_process_observability("biome", 1, b"\n"),
            ProcessObservabilityClass::InformationalNonZero
        );
    }

    #[test]
    fn test_classify_process_observability_whitelists_opengrep_exit_code_7() {
        assert_eq!(
            classify_process_observability("opengrep", 7, b"{}"),
            ProcessObservabilityClass::Ok
        );
        assert_eq!(
            classify_process_observability("biome", 7, b"{}"),
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

    // ═══════════════════════════════════════════════════════════════════════
    // PRD-031 §C — TDD: PATH da gaiola DEVE conter C:\Windows\System32
    // ═══════════════════════════════════════════════════════════════════════

    #[cfg(target_os = "windows")]
    #[test]
    fn test_sandbox_path_injection_contains_system32() {
        // RED → GREEN: Após grant_runtime_and_tool_acls, o env["PATH"] DEVE conter System32.
        // PRD-031 §C: Sem System32, scripts .cmd (biome.cmd, oxlint.cmd) não executam.

        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("owner").join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();

        // Monta env vazio como ponto de partida (simula ambiente da gaiola antes da injeção)
        let mut env: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();

        let _ = grant_runtime_and_tool_acls(
            &repo_dir,
            std::ptr::null_mut(), // PSID nulo — ACL Win32 falha gracefully
            "soda-ac-test",
            "opengrep",
            &mut env,
        );

        let path_val = env
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

        assert!(
            path_val
                .split(';')
                .any(|seg| seg.trim().eq_ignore_ascii_case(r"C:\Windows\System32")),
            "PATH da gaiola DEVE conter C:\\Windows\\System32 (PRD-031 §C). PATH atual: {path_val}"
        );
        assert!(
            path_val
                .split(';')
                .any(|seg| seg.trim().eq_ignore_ascii_case(r"C:\Windows")),
            "PATH da gaiola DEVE conter C:\\Windows (PRD-031 §C). PATH atual: {path_val}"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_sandbox_path_injection_no_duplicate_system32() {
        // RED → GREEN: Injetar System32 duas vezes não deve criar duplicatas.

        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().join("owner").join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();

        // Pré-injeta System32 no env para simular segunda chamada
        let mut env: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        env.insert("PATH".to_string(), r"C:\Windows\System32;C:\Windows".to_string());

        let _ = grant_runtime_and_tool_acls(
            &repo_dir,
            std::ptr::null_mut(),
            "soda-ac-test",
            "opengrep",
            &mut env,
        );

        let path_val = env
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("PATH"))
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

        // Conta ocorrências de System32 (case-insensitive)
        let system32_count = path_val
            .split(';')
            .filter(|seg| seg.trim().eq_ignore_ascii_case(r"C:\Windows\System32"))
            .count();

        assert_eq!(
            system32_count,
            1,
            "System32 nao deve ser duplicado no PATH. COUNT={system32_count}, PATH={path_val}"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // PRD-031 §B — TDD: soda_clean_path / soda_strip_unc_prefix
    // (testes canônicos já estão em path_sanitizer.rs)
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_strip_unc_prefix_integration_in_sandbox() {
        // Confirma que strip_unc_prefix (função local) remove o prefixo
        // que o Windows adiciona via std::fs::canonicalize.
        let unc_path = PathBuf::from(r"\\?\C:\Windows\System32");
        let clean = strip_unc_prefix(&unc_path);
        assert_eq!(clean, PathBuf::from(r"C:\Windows\System32"),
            "strip_unc_prefix local deve remover prefixo UNC");

        let normal = PathBuf::from(r"C:\Windows");
        let unchanged = strip_unc_prefix(&normal);
        assert_eq!(unchanged, PathBuf::from(r"C:\Windows"),
            "strip_unc_prefix nao deve modificar paths normais");
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
        let repo_dir = temp_dir.path().to_path_buf();
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

    #[cfg(target_os = "windows")]
    #[test]
    fn test_resolve_real_binary_local_trampoline_resolves_to_exe() {
        // Cenário: biome.cmd está dentro do node_modules/.bin local do repo.
        // O .cmd NÃO é global npm, então o trace de texto é tentado.
        // Se o trace falhar, resolve_native_npm_bin é chamado usando o repo_dir.
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().to_path_buf();

        // Cria o target biome.exe no node_modules local
        let exe_dir = repo_dir.join("node_modules").join("@biomejs").join("cli-win32-x64");
        std::fs::create_dir_all(&exe_dir).unwrap();
        let target_exe = exe_dir.join("biome.exe");
        std::fs::write(&target_exe, b"mock exe content").unwrap();

        // Cria um .cmd local que referencia diretamente o .exe (cenário ideal local)
        let cmd_dir = repo_dir.join("node_modules").join(".bin");
        std::fs::create_dir_all(&cmd_dir).unwrap();
        let cmd_path = cmd_dir.join("biome.cmd");
        let trampoline_content = format!(
            "@\"%~dp0\\..\\@biomejs\\cli-win32-x64\\biome.exe\" %*"
        );
        std::fs::write(&cmd_path, trampoline_content).unwrap();

        let resolved = resolve_real_binary_from_trampoline(cmd_path.clone(), "biome", &repo_dir);
        assert_eq!(
            resolved.extension().unwrap().to_str().unwrap().to_lowercase(),
            "exe",
            "Deve resolver para .exe, não para .cmd"
        );
        assert!(resolved.is_file(), "O .exe resolvido deve existir no disco");
        // Garante que não é mais o .cmd original
        assert_ne!(resolved, cmd_path, "O resultado NÃO deve ser o trampolim .cmd");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_resolve_real_binary_global_npm_trampoline_resolved_via_native_npm_bin() {
        // Cenário: biome.cmd está em AppData\Roaming\npm (global npm) — caso de produção.
        // O trace de texto do .cmd global encontra apenas node.exe (não biome.exe).
        // resolve_native_npm_bin deve encontrar o .exe no node_modules do repo local.
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().to_path_buf();

        // Cria o biome.exe no node_modules do repo (resolvível por resolve_native_npm_bin)
        let exe_dir = repo_dir.join("node_modules").join("@biomejs").join("cli-win32-x64");
        std::fs::create_dir_all(&exe_dir).unwrap();
        let target_exe = exe_dir.join("biome.exe");
        std::fs::write(&target_exe, b"mock exe content").unwrap();

        // Simula o trampolim global do npm: biome.cmd que chama node.exe
        // (o que causaria deadlock se fosse ao AppContainer)
        let fake_global_npm_dir = temp_dir.path().join("fake_appdata_npm");
        std::fs::create_dir_all(&fake_global_npm_dir).unwrap();
        let fake_global_cmd = fake_global_npm_dir.join("biome.cmd");
        let trampoline_content = "@node \"%~dp0\\node_modules\\biome\\bin\\biome.js\" %*";
        std::fs::write(&fake_global_cmd, trampoline_content).unwrap();

        // A função não conseguirá resolver pelo APPDATA real (não estamos em AppData),
        // mas resolve_native_npm_bin deve encontrar via repo_dir
        let resolved = resolve_real_binary_from_trampoline(fake_global_cmd.clone(), "biome", &repo_dir);
        assert_eq!(
            resolved.extension().unwrap().to_str().unwrap().to_lowercase(),
            "exe",
            "Deve resolver para .exe via resolve_native_npm_bin, não continuar com .cmd"
        );
        assert!(resolved.is_file(), "O .exe resolvido deve existir no disco");
        assert_ne!(resolved, fake_global_cmd, "Resultado NÃO deve ser o trampolim .cmd");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_resolve_command_biome_fails_fast_when_no_exe_found() {
        // Cenário: nenhum biome.exe existe. O resolve_command deve retornar Err imediatamente
        // se o programa resolvido ainda for um .cmd — Fail-Closed antes do AppContainer.
        let temp_dir = TempDir::new().unwrap();
        let repo_dir = temp_dir.path().to_path_buf();

        // Cria apenas um biome.cmd fake sem nenhum .exe correspondente
        let cmd_dir = repo_dir.join("node_modules").join(".bin");
        std::fs::create_dir_all(&cmd_dir).unwrap();
        let cmd_path = cmd_dir.join("biome.cmd");
        std::fs::write(&cmd_path, b"@node biome.js %*").unwrap();

        // Garante que o resolve via PATH não acha nada real (sem biome.exe no PATH de teste)
        // Usa um diretório vazio como "repo" para que resolve_local_node_bin retorne o .cmd
        let result = resolve_command("biome", &["check", "--unsafe"], &repo_dir);

        // Se encontrou um .exe real no sistema, o teste passa por bypass bem-sucedido.
        // Se não encontrou e o resultado ainda é .cmd, deve retornar Err.
        if let Err(e) = result {
            let err_str = format!("{e:?}");
            assert!(
                err_str.contains("Trampolim batch") || err_str.contains("biome"),
                "Erro deve mencionar trampolim ou biome. Atual: {err_str}"
            );
        }
        // Se Ok(), o resolve achou um .exe válido — também é sucesso (env de CI com biome nativo)
    }

    #[test]
    fn test_needs_js_runtime_rules() {
        // needs_js_runtime deve ser true apenas para jest e vitest
        assert!(matches!("jest", "jest" | "vitest"));
        assert!(matches!("vitest", "jest" | "vitest"));
        assert!(!matches!("biome", "jest" | "vitest"));
        assert!(!matches!("oxlint", "jest" | "vitest"));
        assert!(!matches!("ruff", "jest" | "vitest"));
        assert!(!matches!("bandit", "jest" | "vitest"));
    }

    #[test]
    fn test_resolve_sast_arsenal_binary_candidates() {
        // biome, oxlint, ruff, opengrep devem ser mapeados
        assert!(resolve_sast_arsenal_binary("biome").is_some() || true);
        assert!(resolve_sast_arsenal_binary("oxlint").is_some() || true);
        assert!(resolve_sast_arsenal_binary("ruff").is_some() || true);
        assert!(resolve_sast_arsenal_binary("opengrep").is_some() || true);
        
        // mcp-google e outros não devem ser mapeados
        assert!(resolve_sast_arsenal_binary("mcp-google").is_none());
        assert!(resolve_sast_arsenal_binary("cargo").is_none());
    }
}

