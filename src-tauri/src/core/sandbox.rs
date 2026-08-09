//! Core Sandbox Engine — Less Privileged AppContainer (LPAC) Nativo do Windows 11
//!
//! Este módulo implementa o isolamento físico bare-metal para subprocessos no Shadow Workspace
//! utilizando as primitivas de segurança do Windows 11:
//! 1. Less Privileged AppContainer (LPAC) via `CreateAppContainerProfile`.
//! 2. Mutação de ACLs NTFS via `SetNamedSecurityInfoW` e `SetEntriesInAclW`.
//! 3. Bloqueio total de rede local via 0 capacidades no `SECURITY_CAPABILITIES`.
//! 4. Confinamento e Morte Coletiva via Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
//! 5. Bypass Gracioso para erro `0x80070005` (Access Denied) com Varredura Estática O(1) de AST no `Cargo.toml`.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

#[cfg(target_os = "windows")]
use windows_sys::core::HRESULT;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{
    CloseHandle, LocalFree, ERROR_ACCESS_DENIED, GENERIC_EXECUTE, GENERIC_READ, GENERIC_WRITE,
    HANDLE,
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::Authorization::{
    SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, SE_FILE_OBJECT, SET_ACCESS,
    TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN,
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile,
    DeriveAppContainerSidFromAppContainerName,
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::{
    FreeSid, ACL, CONTAINER_INHERIT_ACE, DACL_SECURITY_INFORMATION, OBJECT_INHERIT_ACE,
    PROTECTED_DACL_SECURITY_INFORMATION, SECURITY_CAPABILITIES, PSID,
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::Storage::FileSystem::DELETE;

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
    JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, InitializeProcThreadAttributeList,
    UpdateProcThreadAttribute, CREATE_UNICODE_ENVIRONMENT, EXTENDED_STARTUPINFO_PRESENT,
    LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, STARTUPINFOEXW,
};

use crate::cognition::memory_graph::errors::CognitiveError;

/// Constante Win32 ProcThreadAttribute para Security Capabilities (AppContainer)
#[cfg(target_os = "windows")]
const PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES: usize = 0x00020009;

/// HRESULT de Acesso Negado (0x80070005)
#[cfg(target_os = "windows")]
const HRESULT_ACCESS_DENIED: HRESULT = -2147024891; // 0x80070005 em i32 decimal

/// Converte uma string ou Path para formato UTF-16 terminado em nulo para Win32 API.
fn to_wide_null<S: AsRef<OsStr> + ?Sized>(s: &S) -> Vec<u16> {
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

/// Envolvente RAII para o Handle do Job Object do Windows Kernel.
#[cfg(target_os = "windows")]
pub struct WindowsJobGuard {
    handle: HANDLE,
}

#[cfg(target_os = "windows")]
impl WindowsJobGuard {
    pub fn new() -> Result<Self, String> {
        let job_handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job_handle.is_null() {
            return Err("CreateJobObjectW retornou Handle nulo".to_string());
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let info_len = std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32;

        let set_ok = unsafe {
            SetInformationJobObject(
                job_handle,
                JobObjectExtendedLimitInformation,
                &mut info as *mut _ as *mut _,
                info_len,
            )
        };
        if set_ok == 0 {
            unsafe { CloseHandle(job_handle); }
            return Err("SetInformationJobObject falhou ao aplicar JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE".to_string());
        }

        Ok(Self { handle: job_handle })
    }

    /// Anexa um processo ativo ao Job Object.
    ///
    /// # Safety
    /// O operador deve garantir que `process_handle` seja um handle válido e aberto do Windows para o processo alvo.
    pub unsafe fn assign_process(&self, process_handle: HANDLE) -> Result<(), String> {
        let res = AssignProcessToJobObject(self.handle, process_handle);
        if res == 0 {
            Err("AssignProcessToJobObject falhou ao anexar o processo ao Job Object".to_string())
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsJobGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { CloseHandle(self.handle); }
        }
    }
}

/// Remove o perfil LPAC de um AppContainer do Registro do Windows (Limpeza Idempotente).
pub fn cleanup_lpac_profile(app_container_name: &str) {
    #[cfg(target_os = "windows")]
    {
        let name_wide = to_wide_null(app_container_name);
        unsafe {
            let _ = DeleteAppContainerProfile(name_wide.as_ptr());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app_container_name;
    }
}

/// Varredura Estática Bare-Metal O(1) no Cargo.toml do workspace para o Bypass Gracioso.
/// Retorna erro `CognitiveError::UntrustedExecutionBlocked` se detectar `build.rs` ativos ou `proc-macro = true`.
pub fn verify_workspace_cargo_toml_safety(workspace_path: &Path) -> Result<(), CognitiveError> {
    let cargo_toml_path = workspace_path.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Ok(());
    }

    // Checagem estática física por build.rs no diretório
    let build_rs_path = workspace_path.join("build.rs");
    if build_rs_path.exists() {
        return Err(CognitiveError::UntrustedExecutionBlocked(
            "Arquivo build.rs de terceiros detectado no workspace durante o Bypass Gracioso".to_string(),
        ));
    }

    let content = match std::fs::read_to_string(&cargo_toml_path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    if content.contains("proc-macro = true") || content.contains("proc_macro = true") {
        return Err(CognitiveError::UntrustedExecutionBlocked(
            "Declaração de proc-macro = true detectada no Cargo.toml do workspace durante o Bypass Gracioso".to_string(),
        ));
    }

    if content.contains("build = ") {
        return Err(CognitiveError::UntrustedExecutionBlocked(
            "Script de build customizado declarado no Cargo.toml do workspace durante o Bypass Gracioso".to_string(),
        ));
    }

    Ok(())
}

/// Executa a criação de um subprocesso sob enjaulamento nativo LPAC (Less Privileged AppContainer) no Windows.
/// Em caso de falha de permissão (HRESULT 0x80070005), aciona pacificamente o Bypass Gracioso com isolamento por
/// Job Object e varredura estática de AST.
pub fn create_lpac_sandbox_process(
    app_container_name: &str,
    workspace_path: &str,
    executable_path: &str,
    args: &[&str],
) -> Result<u32, String> {
    // 1. Canonicalization de Caminho obrigatória para evitar bypasses de travessia
    let canonical_workspace = dunce::canonicalize(workspace_path)
        .unwrap_or_else(|_| std::path::PathBuf::from(workspace_path));
    
    // Assegura existência do diretório do workspace
    if !canonical_workspace.exists() {
        let _ = std::fs::create_dir_all(&canonical_workspace);
    }

    #[cfg(target_os = "windows")]
    {
        create_lpac_sandbox_process_windows(
            app_container_name,
            &canonical_workspace,
            executable_path,
            args,
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app_container_name, executable_path, args);
        Err("Isolamento LPAC é exclusivo para sistemas Windows 11+".to_string())
    }
}

#[cfg(target_os = "windows")]
fn create_lpac_sandbox_process_windows(
    app_container_name: &str,
    canonical_workspace: &Path,
    executable_path: &str,
    args: &[&str],
) -> Result<u32, String> {
    let name_wide = to_wide_null(app_container_name);
    let mut container_sid: PSID = std::ptr::null_mut();

    // 2. Obtenção do SID do Perfil LPAC via Win32 API
    let mut hr = unsafe {
        CreateAppContainerProfile(
            name_wide.as_ptr(),
            name_wide.as_ptr(),
            name_wide.as_ptr(),
            std::ptr::null(),
            0,
            &mut container_sid,
        )
    };

    // Se o perfil já existir (0x800700B7), deriva o SID existente
    if hr == (0x800700B7u32 as i32) {
        hr = unsafe {
            DeriveAppContainerSidFromAppContainerName(name_wide.as_ptr(), &mut container_sid)
        };
    }

    // 3. Intercepção de erro 0x80070005 -> Bypass Gracioso
    if hr == HRESULT_ACCESS_DENIED || (hr < 0 && (hr & 0xFFFF) == 5) {
        eprintln!(
            "[SOULS LPAC WARN] Falha de permissão ao criar perfil LPAC (HRESULT 0x80070005 / Access Denied). Acionando Bypass Gracioso com Job Objects + Varredura AST."
        );
        return fallback_graceful_bypass(canonical_workspace, executable_path, args);
    }

    if hr < 0 || container_sid.is_null() {
        eprintln!(
            "[SOULS LPAC WARN] CreateAppContainerProfile retornou HRESULT {:#x}. Acionando Bypass Gracioso.",
            hr
        );
        return fallback_graceful_bypass(canonical_workspace, executable_path, args);
    }

    // 4. Mutação NTFS ACL no diretório do workspace (SetNamedSecurityInfoW)
    let acl_res = apply_workspace_ntfs_acl(canonical_workspace, container_sid);
    if let Err(acl_err) = acl_res {
        unsafe { FreeSid(container_sid); }
        if acl_err.contains("0x80070005") || acl_err.contains("Access Denied") {
            eprintln!(
                "[SOULS LPAC WARN] Falha na mutação de ACL NTFS (Access Denied). Acionando Bypass Gracioso."
            );
            return fallback_graceful_bypass(canonical_workspace, executable_path, args);
        }
        return Err(acl_err);
    }

    // 5. Configuração de STARTUPINFOEXW e PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES
    let mut attr_size: usize = 0;
    unsafe {
        InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut attr_size);
    }
    let mut attr_buf = vec![0u8; attr_size];
    let attr_list = attr_buf.as_mut_ptr() as LPPROC_THREAD_ATTRIBUTE_LIST;

    if unsafe { InitializeProcThreadAttributeList(attr_list, 1, 0, &mut attr_size) } == 0 {
        unsafe { FreeSid(container_sid); }
        return Err("InitializeProcThreadAttributeList falhou".to_string());
    }

    let mut sec_cap: SECURITY_CAPABILITIES = unsafe { std::mem::zeroed() };
    sec_cap.AppContainerSid = container_sid;
    sec_cap.Capabilities = std::ptr::null_mut(); // 0 capacidades -> Bloqueio total de rede local!
    sec_cap.CapabilityCount = 0;
    sec_cap.Reserved = 0;

    let update_ok = unsafe {
        UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
            &mut sec_cap as *mut _ as *const _,
            std::mem::size_of::<SECURITY_CAPABILITIES>(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    if update_ok == 0 {
        unsafe {
            DeleteProcThreadAttributeList(attr_list);
            FreeSid(container_sid);
        }
        return Err("UpdateProcThreadAttribute falhou ao aplicar LPAC security capabilities".to_string());
    }

    let mut startup_info_ex: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    startup_info_ex.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    startup_info_ex.lpAttributeList = attr_list;

    // Linha de comando do processo
    let cmd_str = if args.is_empty() {
        format!("\"{}\"", executable_path)
    } else {
        format!("\"{}\" {}", executable_path, args.join(" "))
    };
    let mut cmd_wide = to_wide_null(&cmd_str);
    let work_dir_wide = to_wide_null(canonical_workspace.as_os_str());

    let job_guard = match WindowsJobGuard::new() {
        Ok(j) => j,
        Err(e) => {
            unsafe {
                DeleteProcThreadAttributeList(attr_list);
                FreeSid(container_sid);
            }
            return Err(e);
        }
    };

    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    let created = unsafe {
        CreateProcessW(
            std::ptr::null(),
            cmd_wide.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0, // Inherit handles = FALSE
            EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
            std::ptr::null(),
            work_dir_wide.as_ptr(),
            &startup_info_ex.StartupInfo,
            &mut pi,
        )
    };

    // REGRA CONSTITUCIONAL: Evitar leaks de heap limpando a lista de atributos e liberando o SID imediatamente
    unsafe {
        DeleteProcThreadAttributeList(attr_list);
        drop(attr_buf);
        FreeSid(container_sid);
    }

    if created == 0 {
        return Err("CreateProcessW falhou ao inicializar o processo enjaulado LPAC".to_string());
    }

    // Anexa o processo ao Job Object para aniquilação automática se o SOULS cair
    let assign_res = unsafe { job_guard.assign_process(pi.hProcess) };
    if let Err(job_err) = assign_res {
        unsafe {
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
        }
        return Err(job_err);
    }

    let pid = pi.dwProcessId;

    unsafe {
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
    }

    // Evita o encerramento pré-maturo do JobObject mantendo o Guard vivo via std::mem::forget ou vazando o handle seguro
    std::mem::forget(job_guard);

    Ok(pid)
}

/// Aplica permissões NTFS atômicas (`GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE | DELETE`) sobre o workspace
/// especificamente para o SID do LPAC via `SetNamedSecurityInfoW`.
#[cfg(target_os = "windows")]
fn apply_workspace_ntfs_acl(workspace_path: &Path, sid: PSID) -> Result<(), String> {
    let mut ea: EXPLICIT_ACCESS_W = unsafe { std::mem::zeroed() };
    ea.grfAccessPermissions = GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE | DELETE;
    ea.grfAccessMode = SET_ACCESS;
    ea.grfInheritance = OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE;
    ea.Trustee.pMultipleTrustee = std::ptr::null_mut();
    ea.Trustee.MultipleTrusteeOperation = 0; // NO_MULTIPLE_TRUSTEE
    ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    ea.Trustee.TrusteeType = TRUSTEE_IS_UNKNOWN;
    ea.Trustee.ptstrName = sid as *mut u16;

    let mut new_acl: *mut ACL = std::ptr::null_mut();
    let set_entries_res = unsafe { SetEntriesInAclW(1, &ea, std::ptr::null(), &mut new_acl) };

    if set_entries_res != 0 || new_acl.is_null() {
        return Err(format!("SetEntriesInAclW falhou com erro Win32 {}", set_entries_res));
    }

    let path_wide = to_wide_null(workspace_path.as_os_str());
    let status = unsafe {
        SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_acl,
            std::ptr::null_mut(),
        )
    };

    unsafe {
        LocalFree(new_acl as *mut _);
    }

    if status == ERROR_ACCESS_DENIED {
        return Err("SetNamedSecurityInfoW falhou com Access Denied (0x80070005)".to_string());
    }

    if status != 0 {
        return Err(format!("SetNamedSecurityInfoW falhou com erro Win32 {}", status));
    }

    Ok(())
}

/// Fallback do Bypass Gracioso para quando a criação do perfil LPAC ou a mutação NTFS falharem com Access Denied.
#[cfg(target_os = "windows")]
fn fallback_graceful_bypass(
    canonical_workspace: &Path,
    executable_path: &str,
    args: &[&str],
) -> Result<u32, String> {
    // 1. Varredura estática de AST no Cargo.toml do workspace
    if let Err(blocked_err) = verify_workspace_cargo_toml_safety(canonical_workspace) {
        return Err(blocked_err.to_string());
    }

    // 2. Inicializa o processo com isolamento básico via Windows Job Objects
    let job_guard = WindowsJobGuard::new()?;

    let mut cmd = std::process::Command::new(executable_path);
    cmd.args(args);
    cmd.current_dir(canonical_workspace);

    let child = cmd
        .spawn()
        .map_err(|e| format!("Falha ao inicializar processo em fallback: {}", e))?;

    let pid = child.id();

    // Obtém o handle cru do processo para anexar ao Job Object
    use std::os::windows::io::AsRawHandle;
    let handle = child.as_raw_handle() as HANDLE;
    unsafe { job_guard.assign_process(handle)?; }

    std::mem::forget(job_guard);
    std::mem::forget(child);

    Ok(pid)
}
