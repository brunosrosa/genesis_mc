# PRD & Spike de Compilação: Atualização de Motores de Inferência Soberanos (SOULS V4)

Este documento estabelece as diretrizes de especificação, fiação de compilação e plano de contingência para o transplante do **Inference Core** do Souls MC na **Arquitetura SOULS V4**.

---

## 1. OBJETIVO DO SPIKE
Garantir o isolamento absoluto e a coexistência harmônica dos 3 motores de inferência primários na máquina local (Intel i9 + RTX 2060m 6GB + CUDA v13.3) sem vazamentos de VRAM, colisões de símbolos de linkagem C-FFI ou pânicos de segmentação no Tokio:
1. **`llama-cpp-2` (Estável - v0.1.153):** Motor principal de inferência contínua persistente na dGPU (Zero-Copy mmap + KV Cache quantizado em Q4_K).
2. **`llama-cpp-turboquant` (Experimental):** Crate de extensão local e isolada para compressão agressiva de KV Cache de 1.5 a 2 bits (TQ1/TQ2) visando contextos de até 32k.
3. **`bitnet.cpp` (Reserva Ternária - 1.58-bits):** Daemon de CPU assíncrono isolado em subprocesso de sistema, com transporte de dados via Memória Compartilhada (SHM) de cópia zero usando `iceoryx2`.

---

## 2. COMPONENTE I: A VENDORIZAÇÃO DO `llama-cpp-2` (Pista de Voo Estável)

### A. Correção de Dependência no `Cargo.toml`
Para evitar travamento em atualizações cegas e obter controle de baixo nível das assinaturas do GGML, a crate oficial será canibalizada para o diretório de terceiros local:

```toml
# src-tauri/Cargo.toml
[dependencies]
# A dependência original aponta para o patch local
llama-cpp-2 = { path = "vendor/llama-cpp-2", optional = true, features = ["cuda"] }

[patch.crates-io]
# Garante que qualquer sub-dependência transitória use a nossa versão purificada
llama-cpp-2 = { path = "vendor/llama-cpp-2" }
```

### B. Heurística de Alocação de KV Cache em `llama_engine.rs`
Na atualização para a versão **`0.1.153`**, as estruturas de inicialização de parâmetros de contexto em Rust sofreram mutações. O novo motor deve ser instanciado respeitando o cálculo determinístico de VRAM em tempo constante $\mathcal{O}(1)$:

```rust
// src-tauri/src/core/llama_engine.rs
use llama_cpp_2::context::params::LlamaContextParams;

pub fn build_context_params_v4(declared_ctx: u32, head_v: u32) -> LlamaContextParams {
    let mut params = LlamaContextParams::default();
    
    // Key Cache fixado em F16 (RoPE exige precisão geométrica)
    params.set_type_k(KvCacheType::F16);
    
    // Value Cache quantizado de forma assimétrica para poupar VRAM
    let type_v = if head_v > 0 && head_v % 256 == 0 {
        KvCacheType::Q4_K
    } else {
        KvCacheType::Q8_0
    };
    params.set_type_v(type_v);
    
    // Vincula a janela máxima estrita
    params.set_n_ctx(Some(declared_ctx));
    
    params
}
```

---

## 3. COMPONENTE II: A EXTENSÃO `llama-cpp-turboquant` (Pista de Voo de Vanguarda)

### A. Prevenção de Colisões de ABI e Alinhamento de Memória
A crate experimental **`llama-cpp-turboquant`** será mantida em um namespace inteiramente separado (`llama_cpp_turboquant`), possuindo seus próprios bindings brutos de FFI (`llama-cpp-sys-turboquant`).

### B. Mapeamento de Tipos Estendidos
Para evitar *Segmentation Faults* em tempo de execução, os novos enums de GGML do TurboQuant devem ser espelhados de forma idêntica entre o compilador C++ (MSVC/NVCC) e o compilador Rust (rustc):

```rust
// vendor/llama-cpp-turboquant/src/types.rs
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TurboQuantCacheType {
    F32 = 0,
    F16 = 1,
    Q4_0 = 2,
    Q4_1 = 3,
    Q8_0 = 8,
    // Tipos customizados injetados no cabeçalho GGML pelo Fork TurboQuant
    GGML_TYPE_TQ1 = 16, // Turbo 1.5-bit
    GGML_TYPE_TQ2 = 17, // Turbo 2.0-bit
}
```

---

## 4. COMPONENTE III: A JAULA DO `bitnet.cpp` (Daemon por SHM IPC)

### A. Isolamento Total contra Conflitos de Linker
Como o `bitnet.cpp` compartilha os mesmos símbolos globais em C da biblioteca GGML padrão (ex: `ggml_init`, `ggml_graph_compute`), o linker dispararia erros de duplicação se ambos fossem agregados no mesmo binário Rust. 

**A Solução de Soberania (ADR-027):** O `bitnet.cpp` rodará como um executável sidecar independente em `resources/bin/bitnet_daemon.exe`.

```
[Souls MC (Rust central)] <─── (iceoryx2 / Zero-Copy SHM Channel) ───> [bitnet_daemon.exe (CPU)]
```

### B. O Orquestrador de Ciclo de Vida do Daemon
O Rust central gerencia síncronamente a criação, telemetria e o desligamento atômico do sidecar ternário na CPU:

```rust
// src-tauri/src/core/bitnet_daemon.rs
use tokio::process::{Command, Child};
use std::process::Stdio;

pub struct BitNetDaemon {
    process: Option<Child>,
    shm_segment_id: String,
}

impl BitNetDaemon {
    pub async fn spawn_isolated_daemon(bin_path: &Path) -> Result<Self, std::io::Error> {
        let segment_id = format!("souls_shm_bitnet_{}", std::process::id());
        
        let child = Command::new(bin_path)
            .arg("--shm-id")
            .arg(&segment_id)
            .arg("--threads")
            .arg("4") // Configura afinidade de threads na CPU
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(Self {
            process: Some(child),
            shm_segment_id: segment_id,
        })
    }

    pub async fn kill_and_purge_vram(&mut self) -> Result<(), std::io::Error> {
        if let Some(mut child) = self.process.take() {
            // Guilhotina Atômica (SIGKILL) para não deixar processos zumbis consumindo RAM
            child.kill().await?;
        }
        Ok(())
    }
}
```

---

## 5. SUÍTE DE TESTES UNITÁRIOS RESTRITA (TDD MANDATÓRIO)

A fim de garantir a conformidade com as diretrizes do SOULS, o agente na IDE deverá obrigatoriamente implementar e passar nos seguintes 3 cenários de validação física:

1. **`test_model_registry_respects_max_depth_5`:** Garante que a varredura do `WalkDir` interrompa a recursão estritamente no 5º nível de pastas.
2. **`test_single_mmap_per_inference`:** Prova que o cabeçalho do arquivo `.gguf` é mapeado na memória virtual e lido apenas uma vez por ciclo de inferência, impedindo redundâncias.
3. **`test_bitnet_lifecycle_failsafe`:** Valida que a destruição da struct `BitNetDaemon` resulta na interrupção física imediata e purga de RAM do subprocesso de CPU no host Windows.
