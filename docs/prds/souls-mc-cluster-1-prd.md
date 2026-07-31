# Requisitos do Produto (PRD): Cluster 1 — Escrita e Edição Cirúrgica (`souls_edit` & `souls_fill`)

## 🏛️ 1. Introdução e Contexto Técnico

No ecossistema do **Souls MC**, os agentes de Inteligência Artificial atuam como operários locais dentro do ambiente do usuário (Host). Permitir que esses agentes executem modificações cegas no código-fonte do projeto (como reescrever arquivos inteiros de milhares de linhas por causa de alterações pontuais) gera um altíssimo custo de processamento, estoura a janela de contexto de forma prematura e viola o **ADR-010 (Spec-Driven Development)**, abrindo espaço para a introdução de *slop* sintático e corrupção silenciosa de dados (SDC) [1305, 1306].

Para consolidar as garras físicas ("Mãos") dos agentes na IDE, o **Cluster 1** introduz duas ferramentas MCP cirúrgicas no servidor central `souls_mcp_server`:
1. **`souls_edit`**: Substituição precisa e segura de blocos de código baseada em blocos `SEARCH`/`REPLACE` [307].
2. **`souls_fill`**: Preenchimento cirúrgico de lacunas de código (*skeletons/stubs*) diretamente sob os offsets corretos da Árvore de Sintaxe Abstrata (AST) [307].

Este documento estabelece as especificações de requisitos, o modelo de concorrência por arquivo, as barreiras de segurança física e a suíte de testes (TDD) para a implementação deste cluster sob a **Arquitetura SOULS V5** [300].

---

## 🛡️ 2. Objetivos de Engenharia e Definições de Sucesso

*   **Zero-Copy e Baixa Latência**: A validação sintática do patch e as substituições em RAM devem ocorrer com latência inferior a **1.0 ms** para arquivos comuns (< 100 KB) [1360].
*   **Imunidade a Race Conditions**: Bloqueio concorrente estrito que impeça dois subagentes efêmeros ou processos de background de gravarem simultaneamente no mesmo arquivo [825].
*   **Fail-Closed à Prova de Falhas**: Se um bloco `SEARCH` fornecido pela IA não corresponder exatamente ao conteúdo físico do arquivo (incluindo quebras de linha e espaços em branco), a operação de gravação deve falhar de forma limpa, retornando um erro detalhado para correção [830, 831].
*   **Proteção de Fronteira Física**: Bloqueio absoluto que proíba operações de modificação em diretórios confidenciais ou fora da pasta raiz do workspace (como `.env`, segredos locais ou pastas de sistema) [1314].

---

## ⚙️ 3. Especificação da Interface MCP (API)

O servidor Rust estenderá as suas capacidades expondo as seguintes assinaturas na matriz de ferramentas do `souls_mcp_server.rs` [1365]:

### Tool A: `souls_edit`
*   **Descrição**: Executa a substituição atômica de um ou mais blocos lógicos delimitados de código no arquivo alvo.
*   **Parâmetros de Entrada**:
    ```json
    {
      "file_path": "src-tauri/src/bin/souls_mcp_server.rs",
      "patches": [
        {
          "search": "fn old_logic() {\n    // ...\n}",
          "replace": "fn new_logic() {\n    // ...\n}"
        }
      ]
    }
    ```
*   **Comportamento do Motor**:
    1. Resolve o caminho absoluto de `file_path`, higienizando e aplicando a validação de barreira de workspace [1314].
    2. Adquire o **Lock assíncrono (Mutex do Tokio)** mapeado para o caminho do arquivo para evitar concorrência [830].
    3. Carrega o conteúdo do arquivo na RAM em uma única leitura linear.
    4. Varre o buffer local procurando correspondência exata de caractere e quebra de linha para o campo `"search"`.
    5. Se todas as correspondências do array de patches forem bem-sucedidas:
        * Substitui no buffer local pelo bloco `"replace"`.
        * Executa um **Atomic Write Swap**: Grava o novo buffer em um arquivo temporário no mesmo diretório (ex: `.file.tmp`) e chama a função nativa do kernel (`fs::rename`) para substituir atomicamente o arquivo original [1349].
        * Libera o Mutex e retorna sucesso.
    6. Se qualquer correspondência falhar:
        * Sofre **Fail-Closed**: Aborta a gravação, descarta o buffer temporário, libera o Mutex e retorna um erro JSON-RPC indicando a incompatibilidade do bloco de busca [830, 831].

### Tool B: `souls_fill`
*   **Descrição**: Injeta um bloco lógico de código funcional diretamente no offset delimitado por um marcador de stub sem violar a casca sintática adjacente [307].
*   **Parâmetros de Entrada**:
    ```json
    {
      "file_path": "src-tauri/src/persist/ssot_injector.rs",
      "stub_marker": "souls-stub: try_load_repo_heuristics_row",
      "code_payload": "fn try_load_repo_heuristics_row() -> Result<(), Error> {\n    // Lógica real\n}"
    }
    ```
*   **Comportamento do Motor**:
    1. Executa as mesmas checagens de fronteira de workspace e adquire o Mutex do arquivo [830, 1314].
    2. Lê o arquivo. Varre o buffer local para localizar o comentário demarcador (ex: `// souls-stub: ...` ou `/* souls-stub: ... */`) [307].
    3. Localiza as coordenadas exatas de início e fim daquela região sintática utilizando o parser AST leve enjaulado (ou em correspondência direta de delimitadores estruturais) [446].
    4. Substitui o stub pelo payload de código fornecido.
    5. Executa a gravação atômica via `fs::rename` no disco [1349].

---

## 🔒 4. Modelo de Concorrência e Proteção do Event Loop (Anti-Starvation)

Para obedecer rigidamente ao **ADR-003** e ao **ADR-028**, as ferramentas de escrita cirúrgica adotarão a seguinte infraestrutura assíncrona baseada no Tokio no backend Rust [1295, 1332]:

### 1. Mutexes de Arquivo Dinâmicos via `DashMap`
Em vez de utilizarmos um único Mutex global pesado (o que asfixiaria o processamento paralelo de arquivos independentes), o servidor Rust gerenciará as travas de arquivo utilizando um mapa de concorrência na RAM do Host [825]:
```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use dashmap::DashMap;

lazy_static! {
    static ref FILE_LOCKS: Arc<DashMap<PathBuf, Arc<Mutex<()>>>> = Arc::new(DashMap::new());
}
```
*   Antes de operar em qualquer arquivo, o manipulador da rota MCP obtém (ou insere se não existir) o `Arc<Mutex<()>>` associado à chave física do caminho absoluto (`PathBuf`) dentro do `FILE_LOCKS`.
*   A aquisição da trava ocorre por meio do `.lock().await` do Tokio. Isso garante que se dois subagentes efêmeros tentarem modificar o mesmo arquivo simultaneamente, o segundo será suspenso no loop de eventos de forma cooperativa, sem travar threads do sistema e sem poluir o disco com colisões [1282].

### 2. Delegação Bloqueante Isolada (`spawn_blocking`)
A operação física de escrita em disco (`std::fs::write` ou `std::fs::rename`) é síncrona no SQLite e nas chamadas nativas do kernel do Windows.
*   Conforme o **ADR-007**, fica **terminantemente proibido** realizar I/O sínclono direto na thread do Event Loop principal do Tokio [1302].
*   Todo o processo de leitura do arquivo original, verificação de strings em RAM, gravação do arquivo `.tmp` e a chamada atômica `fs::rename` serão encapsulados dentro de blocos `tokio::task::spawn_blocking` [1302]. A thread assíncrona principal apenas faz o `.await` da conclusão da tarefa, protegendo as comunicações do Gateway [1302].

---

## 🚫 5. O Firewall de Segurança Física

Para blindar o sistema contra injeções de prompt maliciosas que tentem instruir o agente de IA a vasculhar ou destruir arquivos de sistema ou confidenciais, a camada de Rust no `souls_mcp_server.rs` implementará um Firewall de Caminhos Estrito no momento de resolver as strings recebidas pelo JSON-RPC [1314]:

```rust
fn validate_and_canonicalize_path(user_path: &str) -> Result<PathBuf, RpcError> {
    let workspace_root = std::env::current_dir()
        .map_err(|e| RpcError::new(-32050, "Erro ao obter o workspace root", e))?;
    
    let target_path = workspace_root.join(user_path);
    let canonical_path = target_path.canonicalize().map_err(|e| RpcError::new(
        -32602,
        format!("Caminho inválido ou inexistente: {}", user_path),
        e
    ))?;

    // 1. Verificação de Escape (Directory Traversal)
    if !canonical_path.starts_with(&workspace_root) {
        return Err(RpcError::new_with_message(
            -32602,
            "Acesso negado: Tentativa de travessia de diretório fora do workspace."
        ));
    }

    // 2. Proteção de Arquivos Críticos de Configuração
    if let Some(file_name) = canonical_path.file_name() {
        let name_str = file_name.to_string_lossy().to_lowercase();
        if name_str == ".env" || name_str.ends_with(".db") || name_str.contains("key") {
            return Err(RpcError::new_with_message(
                -32602,
                "Acesso negado: Modificação física de arquivos de credenciais ou bancos de dados proibida via souls_edit."
            ));
        }
    }

    Ok(canonical_path)
}
```

---

## 🧪 6. Suíte de Testes (TDD / DoD)

Para homologar a finalização do **Cluster 1**, o código de escrita deve bater uma bateria de testes unitários rígida. O Definition of Done (DoD) exige **Exit Code 0** nas seguintes validações [1305]:

### Testes Mandatórios a Implementar:
1.  **`test_edit_successful_patch`**: Cria um arquivo de texto mock temporário em scratchpad, executa `souls_edit` com correspondência exata do bloco `SEARCH` e valida que a alteração foi persistida atomicamente com sucesso [1352].
2.  **`test_edit_fail_closed_on_mismatch`**: Fornece um bloco `SEARCH` que desvia ligeiramente do conteúdo do arquivo de texto mock (ex: uma quebra de linha faltante ou espaço duplo). O teste deve falhar de forma controlada com o erro específico, provando que o arquivo original permaneceu intocado em disco [830].
3.  **`test_concurrency_file_locking`**: Dispara simultaneamente duas requisições paralelas via Tokio de escrita no mesmo arquivo simulando dois subagentes e comprova que o `FILE_LOCKS` em `DashMap` sequenciou as gravações de forma impecável, com zero corrupções [825].
4.  **`test_firewall_directory_traversal`**: Tenta forçar uma alteração passando caminhos com escapes de diretório (ex: `../../../../Windows/System32/drivers/etc/hosts`). O teste deve capturar a rejeição imediata da chamada pelo Firewall de Caminhos, retornando o erro `-32602` [1314].
