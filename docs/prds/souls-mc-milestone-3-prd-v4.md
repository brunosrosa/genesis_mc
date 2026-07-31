# PRD - Universalização AST, Concorrência de Escrita & Execução Elástica (V4)

## 1. Visão Geral
Este documento estabelece as especificações técnicas, contratos lógicos e restrições de baixo nível para a Milestone 3 (V3/V4) do SOULS. O escopo contempla a implementação das rotinas de modificação cirúrgica de arquivos, controle elástico de contexto ciente de VRAM/orçamento de tokens, compilação/teste de terminal assíncrono com supressão de logs ruidosos, e a blindagem de segurança para interceptação e tratamento fail-soft de pânicos originados por FFI C++.

## 2. Requisitos Técnicos & Contratos de Baixo Nível

### A. Cluster de Escrita Cirúrgica (`file_locker.rs` & `souls_edit`)
Para prevenir a Corrupção Silenciosa de Dados (SDC) e colisões causadas por sub-agentes concorrentes tentando editar simultaneamente os mesmos arquivos em disco:
1. **Trava Concorrente Baseada em Mutex:**
   - Criar uma estrutura de dados de persistência global e thread-safe: `PATH_LOCKS: OnceLock<DashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>`.
   - Implementar a função `pub fn acquire_file_lock(path: &Path) -> Arc<tokio::sync::Mutex<()>>`.
   - **Prevenção contra Vazamento de Memória (Memory Leak Guard):** A cada aquisição, verificar se o Mutex inserido não é mais disputado. Para evitar o crescimento monotônico do mapa na RAM ao longo de execuções de longa duração, implementar um coletor de chaves órfãs: remover do `DashMap` as chaves cuja contagem de referências fortes do `Arc` seja igual a 1 (`Arc::strong_count(&lock) == 1`).
2. **Escrita Atômica Protegida (Swap-on-Success):**
   - Implementar `pub async fn atomic_write_file(path: &Path, content: &str) -> Result<(), std::io::Error>`.
   - O conteúdo deve ser gravado primeiramente em um arquivo temporário no mesmo diretório do alvo (`{filename}.tmp_uuid`).
   - Somente se a gravação de bytes no arquivo temporário for bem-sucedida, executar o swap atômico no sistema de arquivos usando a chamada nativa de sistema via `std::fs::rename`. Isso garante tolerância a falhas térmicas, de hardware e quedas de energia.

### B. Preenchimento de Contexto Elástico (`souls_fill`)
O algoritmo de preenchimento atua como o porteiro de FinOps e VRAM:
1. **Verificação de Headroom e Orçamento de Tokens:**
   - Antes de injetar arquivos de rascunho ou contextos de manuais (.cursorrules, CLAUDE.md) nas requisições, calcular o volume de tokens via codificação lexer.
   - Cruzar os dados com os limites físicos cadastrados para o modelo em uso na tabela `model_registry` do SQLite `souls_heuristic_vault.db`.
2. **Poda Semântica Seca (CodeCompressor):**
   - Se o tamanho acumulado exceder a "Zona Vermelha" (80% do limite de contexto do modelo), o `souls_fill` invocará síncronamente o `CodeCompressor` (poda estrutural AST via tree-sitter em Rust) para remover espaços extras, quebras de linha e corpos de métodos irrelevantes, condensando as definições antes do despacho.

### C. Cluster de Execução Elástica (`souls_shell`)
Substituir chamadas de terminal síncronas bloqueantes por um protocolo de suspensão reativa:
1. **Despacho Assíncrono com Tokio Process:**
   - Banir o uso de threads nativas do SO via `std::thread::spawn` que geram desperdício e overhead de CPU no host.
   - Implementar a execução do terminal de forma 100% assíncrona utilizando `tokio::process::Command`.
2. **Isolamento e Redirecionamento de Pipes (MCP Compliance):**
   - **Regra Inegociável (ADR-003):** O `stdout` do processo principal do Souls MC é restrito exclusivamente à comunicação do protocolo JSON-RPC. O vazamento de qualquer caractere de subprocessos (como stdout bruto de um `cargo build`) corromperá a sessão MCP da IDE, derrubando-a.
   - Configurar o `tokio::process::Command` para redirecionar explicitamente `stdout` e `stderr` para `Stdio::piped()`. 
   - Ler os bytes de saída na memória e redirecionar logs de telemetria física unicamente para o canal de erro padrão `stderr` do hospedeiro, ou formatar a resposta para trafegar encapsulada na chave de conteúdo do JSON-RPC.
3. **Pattern Log Compression (Poda de Logs do Compilador):**
   - Implementar a função `pub fn compress_cmd_logs(raw: &str) -> String`.
   - Utilizar expressões regulares eficientes para identificar e remover até 90% dos warnings e mensagens cosméticas ruidosas do cargo/clippy.
   - Preservar intactas as assinaturas de erros sintáticos, números de linhas, caminhos de arquivos e rastreios de pânico (panic stacktraces), entregando uma observação desidratada que caiba perfeitamente no contexto do agente.

### D. Interceptação de Crash de FFI & Bloqueio de Fallback In-Process
A blindagem contra instabilidades lógicas e falhas graves de loaders em C++ (como o crash `invalid vector subscript` do `nemotron_h` de 42 camadas no `llama.cpp` upstream):
1. **Detecção no Nível do Processo Worker:**
   - O processo pai do Tokio monitora o encerramento do subprocesso isolado `souls_vanguard_worker.exe`.
   - Se o worker falhar com um sinal de pânico, crash de acesso à memória (SEH `0xc0000005` ou `std::terminate`), o Rust pai intercepta o pipe quebrado e converte em `InferenceError::ExecutionError`.
2. **Atualização com SQLite Guardrail (Prevenção de SQLITE_BUSY):**
   - O orquestrador deve atualizar a tabela `model_registry` no banco de dados SQLite para desativar o modelo defeituoso (`is_active = 0`).
   - Para evitar bloqueios por concorrência de escrita com outras threads do sistema (`SQLITE_BUSY`), a conexão com o banco de dados deve configurar obrigatoriamente um tempo de espera de recuperação (`busy_timeout(Duration::from_secs(5))`) e operar no modo WAL.
3. **Banimento de Fallback In-Process:**
   - Se a falha do worker for identificada como uma falha fatal de carregamento de tensores ou incompatibilidade de GGUF, o `EngineCascade` **está terminantemente proibido de tentar realizar um fallback automático in-process** no `LlamaCppEngine` hospedeiro. Isso protege o Tokio pai de sofrer o mesmo abort do C++ que aniquilaria o SO agêntico. O erro é retornado de forma fail-soft tipada em Rust.
