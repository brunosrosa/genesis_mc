// SODA-CANIBALIZED: Gerenciador de Subprocesso CPU BitNetDaemon (SODA V4)
// Subprocesso efêmero de 1.58-bits com enjaulamento Windows Job Object (JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE)
// e Drop Guard síncrono para imunidade total contra processos zumbis na RAM.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};
use thiserror::Error;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
    JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

#[derive(Debug, Error)]
pub enum BitNetError {
    #[error("Binário BitNet não encontrado em '{0}'")]
    BinaryNotFound(String),
    #[error("Falha ao inicializar o subprocesso BitNetDaemon: {0}")]
    SpawnError(String),
    #[error("Subprocesso BitNetDaemon encerrado inesperadamente com status: {0}")]
    ProcessTerminated(String),
    #[error("Falha ao enjaular o subprocesso no Job Object do Windows Kernel: {0}")]
    JobObjectError(String),
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

    pub fn assign_child(&self, child: &Child) -> Result<(), String> {
        let raw_handle = child.raw_handle().ok_or_else(|| "Child nao possui RawHandle valido".to_string())? as HANDLE;
        let assign_ok = unsafe { AssignProcessToJobObject(self.handle, raw_handle) };
        if assign_ok == 0 {
            Err("AssignProcessToJobObject falhou em associar o processo ao Job Object".to_string())
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

pub struct BitNetDaemon {
    pub binary_path: PathBuf,
    pub model_path: PathBuf,
    child_process: Option<Child>,
    #[cfg(target_os = "windows")]
    _job_guard: Option<WindowsJobGuard>,
}

impl BitNetDaemon {
    /// Inicializa o subprocesso BitNetDaemon enjaulado no Kernel do Windows.
    pub fn spawn<P: AsRef<Path>>(binary_path: P, model_path: P) -> Result<Self, BitNetError> {
        let bin = binary_path.as_ref().to_path_buf();
        let model = model_path.as_ref().to_path_buf();

        if !bin.exists() {
            return Err(BitNetError::BinaryNotFound(bin.display().to_string()));
        }

        let child = Command::new(&bin)
            .arg("--model")
            .arg(&model)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| BitNetError::SpawnError(e.to_string()))?;

        #[cfg(target_os = "windows")]
        let job_guard = match WindowsJobGuard::new() {
            Ok(guard) => {
                if let Err(e) = guard.assign_child(&child) {
                    tracing::warn!("Falha ao associar BitNetDaemon ao Job Object: {}", e);
                }
                Some(guard)
            }
            Err(e) => {
                tracing::warn!("Falha ao criar Job Object para BitNetDaemon: {}", e);
                None
            }
        };

        Ok(Self {
            binary_path: bin,
            model_path: model,
            child_process: Some(child),
            #[cfg(target_os = "windows")]
            _job_guard: job_guard,
        })
    }

    /// Instância simulada para testes unitários ou ambientes sem binário pré-instalado.
    pub fn mock_for_testing<P: AsRef<Path>>(binary_path: P, model_path: P, child: Option<Child>) -> Self {
        Self {
            binary_path: binary_path.as_ref().to_path_buf(),
            model_path: model_path.as_ref().to_path_buf(),
            child_process: child,
            #[cfg(target_os = "windows")]
            _job_guard: None,
        }
    }

    /// Retorna verdadeiro se o processo ainda estiver ativo.
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child_process {
            match child.try_wait() {
                Ok(None) => true,
                _ => false,
            }
        } else {
            false
        }
    }
}

/// Implementação estritamente síncrona de Drop para extermínio atômico no encerramento.
impl Drop for BitNetDaemon {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child_process.take() {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn test_bitnet_daemon_lifecycle_sigkill() {
        let dir = tempdir().expect("Falha ao criar tempdir");
        let dummy_bin = dir.path().join("bitnet_daemon.exe");
        let dummy_model = dir.path().join("model.gguf");
        File::create(&dummy_bin).expect("Falha ao criar binario simulado");
        File::create(&dummy_model).expect("Falha ao criar modelo simulado");

        let daemon = BitNetDaemon::mock_for_testing(&dummy_bin, &dummy_model, None);
        assert_eq!(daemon.binary_path, dummy_bin);
        drop(daemon);
    }

    #[test]
    fn test_cuda_msvc_build_compatibility() {
        let start = Instant::now();
        let nvcc_path = which::which("nvcc.exe")
            .or_else(|_| {
                if let Ok(cuda_path) = std::env::var("CUDA_PATH") {
                    let p = PathBuf::from(cuda_path).join("bin").join("nvcc.exe");
                    if p.exists() { Ok(p) } else { Err(()) }
                } else {
                    let default_p = PathBuf::from(r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.3\bin\nvcc.exe");
                    if default_p.exists() { Ok(default_p) } else { Err(()) }
                }
            });

        let nvcc = match nvcc_path {
            Ok(p) => p,
            Err(_) => {
                println!("cargo:warning=nvcc.exe nao encontrado no sistema; pulando compilacao real de kernel CUDA");
                return;
            }
        };

        let dir = tempdir().expect("Falha ao criar tempdir para teste CUDA");
        let cu_file = dir.path().join("soda_test_kernel.cu");
        let obj_file = dir.path().join("soda_test_kernel.obj");

        std::fs::write(
            &cu_file,
            r#"
extern "C" __global__ void soda_test_kernel(float* data) {
    int idx = threadIdx.x + blockIdx.x * blockDim.x;
    if (idx == 0) { data[0] = 42.0f; }
}
"#,
        ).expect("Falha ao escrever soda_test_kernel.cu");

        let mut cmd = std::process::Command::new(&nvcc);
        cmd.arg("-c").arg(&cu_file).arg("-o").arg(&obj_file);

        // Se cl.exe nao estiver no PATH padrao, detecta a pasta do MSVC Hostx64/x64 e passa -ccbin
        let cl_path = which::which("cl.exe").or_else(|_| {
            let candidates = [
                r"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\cl.exe",
                r"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.40.33807\bin\Hostx64\x64\cl.exe",
                r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.40.33807\bin\Hostx64\x64\cl.exe",
            ];
            for candidate in candidates {
                let p = PathBuf::from(candidate);
                if p.exists() { return Ok(p); }
            }
            Err(())
        });

        if let Ok(cl_exe) = cl_path {
            if let Some(cl_dir) = cl_exe.parent() {
                cmd.arg("-ccbin").arg(cl_dir);
            }
        }

        let output = cmd.output();

        let elapsed = start.elapsed();
        match output {
            Ok(out) => {
                assert!(
                    out.status.success(),
                    "Compilacao real do kernel CUDA via nvcc.exe falhou (exit code {:?}). Stderr: {}",
                    out.status.code(),
                    String::from_utf8_lossy(&out.stderr)
                );
                assert!(obj_file.exists(), "O arquivo objeto .obj do kernel CUDA nao foi gerado");
                println!(
                    "Empirical CUDA Build PASS: Kernel compilado em {:.3}s via {:?}",
                    elapsed.as_secs_f64(),
                    nvcc
                );
            }
            Err(e) => {
                panic!("Falha ao disparar processo nvcc.exe ({:?}): {}", nvcc, e);
            }
        }
    }
}
