---
id: "ADR-030"
title: "ADR-030-Doutrina-de-Curadoria-de-Dependencias-e-Higiene-de-Crates"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Doutrina implacável de higiene bare-metal: institui o Version Pinning forçado, o extermínio do syn no runtime, a morte do C-FFI criptográfico (ring) e a desidratação de I/O."
---

### ADR-030: Doutrina de Curadoria de Dependências e Higiene de Crates (Rust)

#### Status
Aceito / Consolidado (Ativo, Inegociável e Fundacional para SOULS V4 - Consolidado no Marco 4.2.2)

#### Contexto Técnico e Restrições Físicas de Infraestrutura
A geometria do ecossistema SOULS V4 opera em uma "arquitetura de sobrevivência" atrelada aos limites físicos do hardware host (Intel i9, 32GB RAM, dGPU RTX 2060m com 6GB VRAM). 
Se o gerenciamento de dependências (`Cargo.toml`) for delegado sem restrições a agentes de IA ou processos automatizados, o sistema sofrerá de "Câncer de Dependências": a introdução silenciosa de *frameworks* obesos, C-FFI (Foreign Function Interfaces pesadas), sub-dependências duplicadas (esquizofrenia de RAM) e metadados de macros que causam *Cache Thrashing* no Cache L3 do processador. O tempo de compilação incremental degrada e o *Event Loop* do Tokio entra em colapso termodinâmico.

#### Decisão Arquitetural (As Quatro Leis de Higiene Sistêmica)

Fica decretada a adoção de quatro módulos inegociáveis de curadoria de *crates* para todo o *workspace* Rust do SOULS:

**Módulo 1: O Fim do Bloat de Rede, HTTP e I/O**
*   **Proibição Terminal de Frameworks Obesos:** Ficam sumariamente banidas *crates* como `axum` e `octocrab`. A comunicação de APIs e o roteamento devem ser minimalistas.
*   **Desidratação do `reqwest`:** O cliente HTTP assíncrono `reqwest` está restrito ao uso da *flag* `default-features = false`. É OBRIGATÓRIA a injeção da *feature* `rustls-no-provider` para amputar silenciosamente dependências como o `aws-lc-rs` (que puxa CMake/C) e o `icu4x` / `idna` (tabelas mastodônticas de formatação de idiomas e fusos horários). O `reqwest` operará como um executor HTTP cego e ultraleve.

**Módulo 2: Purificação Criptográfica e Matemática Zero-Copy**
*   **A Morte do `ring` e Esquizofrenia de Hash:** É proibido o uso da biblioteca `ring` devido à sua bagagem C-FFI pesada. A criptografia será operada pelo `rscrypto` via instruções nativas AVX2.
*   **O Dogma da Entropia Unificada:** O ecossistema fará a transição absoluta para a *crate* `rustc-hash v2`, banindo bibliotecas legadas de dicionário (`indexmap v1`). O uso de `rand` genérico para entropia fica obsoleto, adotando-se o gerador na CPU `tinyrand`.

**Módulo 3: O Extermínio Sintático (`syn` no Runtime) e Erradicação de Wrappers Legados**
*   **Banimento do `syn`:** A *crate* `syn` é estritamente proibida no tempo de execução (*runtime*). Ela engole corpos inteiros de funções, gerando picos de *Out-Of-Memory* e tempos de compilação absurdos. O seu uso fica enjaulado exclusivamente na fase de compilação (`proc-macros`).
*   **Banimento da Crate `core_affinity` e `winapi`:** A crate `core_affinity` e wrappers legados C-FFI WinAPI são estritamente banidos do código de produção.
*   **Padronização de Erros:** O mapeamento de falhas fica unificado pela diretiva estável do `thiserror v2` (#![no_std]), eliminando overhead de formatação dinâmica de strings (*Garbage Collection*) no Heap central.

**Módulo 4: O Dogma Hermético do Version Pinning Dinâmico (Marco 4.2.2 Consolidação)**
*   Fica terminantemente proibido o uso de operadores de atualização frouxa (`^`, `*`, `~`) no manifesto de dependências do *Workspace*.
*   Impõe-se a ancoragem estrita e o bloqueio terminal via operador literal de igualdade (`=`) centralizado unicamente no manifesto `Cargo.toml`.
*   **Consolidação no Marco 4.2.2:**
    * `windows-sys` foi unificado e rigidamente pinado na versão `=0.61.2`, aplanando as bindings de sistema operacional com o Tokio 1.51, Gitoxide, tempfile e which.
    * `thiserror` foi promovido e pinado na versão `=2.0.19`.
    * `toml` foi promovido e pinado na versão `=1.1.4`.

#### Snippet Oficial SODA: Ancoragem Nativa de CPU Threads (SetThreadAffinityMask)
Para pinagem de threads críticas de CPU de inferência e workers sem dependências de terceiros (`core_affinity`), adota-se formalmente o padrão nativo via `windows-sys 0.61.2`:

```rust
#[cfg(target_os = "windows")]
{
    use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadAffinityMask};
    let handle = unsafe { GetCurrentThread() };
    for &idx in allowed_core_indices {
        if idx < (usize::BITS as usize) {
            let mask = 1usize << idx;
            let res = unsafe { SetThreadAffinityMask(handle, mask) };
            if res != 0 {
                pinned_indices.push(idx);
            }
        }
    }
}
```

#### Consequências Operacionais e Defesa contra o Slop (Trade-offs)
*   **Positivas:** Redução brutal do tempo de compilação a frio, eliminação da duplicação da stack do `windows-sys` em memória RAM, neutralização de conflitos pelo Cargo e proteção contra alucinação de crates legadas.
*   **Negativas:** Manutenção manual severa. O custo das atualizações de segurança é repassado ao Arquiteto-Chefe.
