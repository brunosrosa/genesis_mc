# PRD: Souls MC - Milestone 3 (V3)
## Cluster de Escrita Cirúrgica, Execução Elástica & Guardas de Inferência

---

## 1. Visão Geral e Justificativa
Este documento estabelece as especificações e contratos imutáveis para as rotinas do **Marco 3 (V3)** do **SODA / Souls MC**. O objetivo principal é consolidar as capacidades operacionais de escrita estável, compilação assíncrona, desidratação de logs, além de sanar preventivamente as vulnerabilidades de runtime e asfixia de contexto introduzidas pela concorrência desordenada de sub-agentes de IA.

---

## 2. Requisitos Técnicos e Contratos

### A. Opção A: O Cluster de Escrita Cirúrgica (`souls_edit` e `souls_fill`)

#### 1. Mutex Assíncrono por Caminho de Arquivo (`souls_edit`)
*   **O Problema:** Se múltiplos sub-agentes tentarem modificar o mesmo arquivo simultaneamente (ex: `Cargo.toml`), ocorrerá colisão de escrita concorrente, resultando em perda de dados ou corrupção sintática silenciosa.
*   **O Contrato:** 
    - Implementar uma tabela global thread-safe e estática: `OnceLock<DashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>` para gerenciar travas de arquivos.
    - Toda chamada a `souls_edit(path, old, new)` deve requisitar e aguardar o bloqueio específico do caminho do arquivo.
    - A gravação física em disco deve ser estritamente atômica utilizando `atomic-write-file` (ou gravação em arquivo temporário com posterior swap e renomeação atômica do sistema operacional via `fs::rename`), impedindo escritas parciais em caso de travamentos.

#### 2. Preenchimento Dinâmico de Contexto FinOps (`souls_fill`)
*   **O Problema:** Agentes que tentam ler ou injetar contextos gigantescos na dGPU estouram o limite físico de VRAM (6GB) da placa base RTX 2060m, gerando spillover PCIe e queda drástica de latência.
*   **O Contrato:**
    - O `souls_fill(path, data)` deve interrogar os metadados do `model_registry` no SQLite para calcular os limites atuais do modelo ativo.
    - Se a janela de contexto estiver na "Zona Vermelha" (> 80% do teto máximo de tokens), o sistema invocará dinamicamente o `CodeCompressor` ou o nosso lexer sínclono de fatiamento sintático (`lean_vacuum`) para expurgar resíduos (comentários verbosos, strings duplicadas, imports redundantes) antes de despachar o payload para a API.

---

### B. Opção B: O Cluster de Execução Elástica (`souls_shell`)

#### 1. Desbaste e Desvio compilatório
*   **O Problema:** Execuções síncronas de terminal (como compilar o projeto com `cargo test`) bloqueiam as threads principais do pool de concorrência do Tokio, gerando latências de cauda imprevisíveis.
*   **O Contrato:**
    - O comando `souls_shell(command)` suspenderá temporariamente a execução do agente executor (pausando o consumo de tokens).
    - O processo filho será disparado em uma thread de sistema isolada via `std::thread::spawn` (fora do Event Loop assíncrono principal).

#### 2. Pattern Log Compression (Poda de Logs)
*   **O Problema:** O output bruto de compilação ou execução de testes do Cargo pode conter centenas de linhas de logs verbosos ou warnings cosméticos que estouram a janela de contexto da IA.
*   **O Contrato:**
    - Implementar um filtro em Rust que consome o output em tempo real.
    - O filtro remove 90% das linhas irrelevantes e warnings do Clippy.
    - Ele preserva e formata de forma limpa apenas as assinaturas exatas e as linhas físicas onde as falhas de testes ou erros de compilação ocorreram, reduzindo o payload final para menos de 10% do tamanho original.

---

### C. Guardas e Curas do Motor de Inferência (Anti-Crash)

#### 1. O Guardião de Fallback In-Process (Prevenção de Crashes do Host)
*   **O Problema:** Se o worker isolado (`souls_vanguard_worker.exe`) crashar ao tentar carregar um modelo devido a um erro profundo de FFI C++ (como a exceção `invalid vector subscript` do Nemotron-3-Nano), a tentativa de realizar fallback chamando o modelo in-process no `LlamaCppEngine` causará o mesmo abort no processo pai, derrubando todo o sistema operacional do Souls MC.
*   **O Contrato:**
    - O `EngineCascade` deve analisar o código do erro. Se o encerramento do worker for classificado como uma falha de inicialização de tensores ou erro de arquitetura de modelo (`null result from llama cpp`), o fallback in-process é **expressamente proibido**.
    - O modelo defeituoso deve ser marcado como `REPROVADO` no SQLite SSOT e o erro deve ser retornado graciosamente como `InferenceError::ExecutionError`, protegendo o processo hospedeiro do Tokio.

#### 2. Otimização de Alocação de IPC (Zero-Allocation JSON-RPC Parsing)
*   **O Problema:** Trafegar payloads extensos em formato JSON através de stdio comum gera milhares de alocações e pressões de Garbage Collection.
*   **O Contrato:**
    - Forçar a leitura e o parsing do JSON do lado do worker de inferência de maneira otimizada (utilizando buffers estáticos pré-alocados ou parsing parcial com `sonic-rs` / `serde_json::from_slice` diretamente sobre as fatias de bytes, evitando a alocação de novas strings dinâmicas na Heap).

---

## 3. Critérios de Aceite e Testes (DoD)
Para homologar a Milestone 3 (V3), a suíte de testes deve provar as seguintes premissas:
1. `test_atomic_souls_edit_concurrency`: Injetar 5 threads concorrentes tentando escrever dados divergentes no mesmo arquivo. Validar que o Mutex as serializou e o arquivo final preservou a integridade física sem perda de caracteres.
2. `test_souls_fill_vram_awareness`: Mockar um limite estrito de tokens e validar que o compressor sínclono reduziu o texto exatamente para caber na zona segura.
3. `test_safe_fallback_guardrail`: Forçar uma falha de carregamento de modelo no worker e assegurar que o processo principal Tokio capturou o erro graciosamente sem tentar a execução in-process, preservando a estabilidade da máquina host.
