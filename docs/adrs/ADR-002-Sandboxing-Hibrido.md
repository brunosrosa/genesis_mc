# ADR-002-Sandboxing-Hibrido

## Status
Aceito (Ativo e Inegociável)

## Contexto
O SODA permite que agentes de IA gerem e executem código, além de interagir com ferramentas externas de terceiros via barramento MCP. Executar código dinamicamente gerado ou bibliotecas não auditadas diretamente no sistema do usuário viola o princípio de segurança Zero-Trust e abre brechas críticas de segurança (ex: execução remota de código - RCE, leitura e deleção arbitrária de arquivos do host). O isolamento não pode introduzir hypervisores pesados ou VMs lentas (ex: QEMU, Firecracker) que asfixiem a CPU i9 e devorem a RAM de 32GB.

## Decisão
Implementar uma arquitetura de **Sandboxing Híbrido Tripartite** governada pelo núcleo Rust:
1. **Lógicas Puras e Scripts Leves (Sem Estado):** Rodam estritamente dentro da engine **Wasmtime (WASI v0.2/v0.3)** compilada nativamente no core. A sandbox Wasm possui acesso zero a disco ou rede do host, exceto via canais IPC virtuais explícitos.
2. **Sidecars Efêmeros Pesados e Binários do Host:** Processos que precisam interagir com recursos físicos ou bibliotecas complexas (ex: Python OCR/Docling) rodam enjaulados pelo Sandboxing nativo do Kernel:
   - **Windows:** Isolamento rigoroso via **AppContainer e LPAC (Low Privilege AppContainer)** (utilizando a crate `rappct`), associado a Windows Job Objects para limitar recursos de CPU/RAM.
   - **Linux:** Enjaulamento via **Landlock LSM** e namespaces (`unshare`), associado a `Cgroups v2`.
3. **Guilhotina Atômica:** Todo sidecar gerado implementa o padrão de `Drop trait` do Rust. Quando a tarefa finaliza, sofre timeout ou o canal IPC é quebrado, o core Rust emite um sinal atômico `SIGKILL` garantindo que nenhum processo zumbi sobreviva na RAM.

## Consequências
- **Segurança Robusta:** Códigos gerados por IA e ferramentas MCP externas não possuem permissões físicas para ler ou corromper arquivos fora das pastas autorizadas.
- **Latência de Inicialização:** Wasmtime inicia sandboxes em menos de **5ms**, e sidecars AppContainer/Landlock herdam o tempo de fork nativo do kernel (~10ms), dispensando o atraso de boot de micro-VMs convencionais.
- **Rigor de Desenvolvimento:** Bibliotecas de terceiros complexas não podem ser injetadas levianamente; devem ser compiladas para WASM ou devidamente encapsuladas nos manifestos de capacidade do sidecar.

## Restrições Bare-Metal
- **Teto de RAM por Sandbox Wasm:** Máximo de **256MB** alocados estritamente na RAM, gerenciados por limites de memória da engine Wasmtime.
- **Teto de CPU por Sidecar:** Limitação física a no máximo **2 threads lógicas** via Job Objects/Cgroups para impedir o congelamento das threads críticas de UI e inferência na CPU i9.
- **Terminação:** O ciclo de vida do sidecar é monitorado por um watchdog assíncrono do Tokio; o `SIGKILL` deve ser disparado em no máximo **100ms** após o timeout da tarefa.
- **Escopo do Wasmtime (WASI 0.2):** Wasmtime é exclusivo para lógica pura sem estado; ferramentas pesadas (ex: Python/PyTorch/OCR) são proibidas em Wasmtime e devem rodar em Micro-VMs.
- **Micro-VMs (Clone VMM / CoW):** Ferramentas pesadas devem usar Micro-VMs KVM via **Clone VMM (Copy-on-Write)**, permitindo forks pré-aquecidos em $< 20ms$ compartilhando RAM.
- **GCR (Shadow Execution):** É obrigatório usar **Shadow Execution** na CPU hospedeira via `cgroups v2` para rastrear matematicamente as *dirty pages* da VRAM e transferir apenas o delta mínimo; abordagens ingênuas de checkpoint/restore que trafeguem grandes volumes via DDR4 são proibidas.
