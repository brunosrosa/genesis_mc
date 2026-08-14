# ESPECIFICAÇÃO TÉCNICA E CADERNO DE TDD: MARCO 5.14.0 (TASK 138)

## 📡 1. Task 138 — Saneamento do Batedor Reativo de Drift (Fase -1)

### 1.1 Racional de Design (A Cura do Boot Bloqueante)
Até as últimas refatorações, o batedor da **Fase -1 (Update/Drift Checker)** realizava chamadas de rede de forma síncrona e sínclona durante o bootstrap (boot) inicial do servidor `souls_mcp_server`. Essa abordagem violava as diretrizes de desenvolvimento local-first e offline do SODA: caso a máquina estivesse offline ou as cotas de taxa (rate limits) de APIs como a do GitHub estivessem estouradas, a inicialização do daemon falhava ou entrava em congelamento severo.

Esta especificação transforma o batedor da Fase -1 em um **Trabalhador Reativo Guiado por Estado (State-Driven Worker)**, estabelecendo o seguinte comportamento:
1. **Boot Síncrono 100% Offline-First**: O boot do servidor de banco de dados e do gateway MCP ocorre estritamente na CPU Host sem tocar na interface de rede.
2. **Desacoplamento por Cronjob Reativo**: A verificação de Drift e atualizações online é delegada a um loop assíncrono em background instanciado sob um cronômetro do Tokio (`tokio::time::interval`).
3. **Detecção Inteligente de Conectividade**: O batedor verifica a presença de internet ativa de forma não-bloqueante utilizando conexões rápidas e de baixa latência em soquetes UDP locais. Caso esteja offline, o sistema silencia o olheiro, preservando ciclos de clock e logs.
4. **Cálculo de Desvio (Drift) O(1)**: O sistema lê a versão local (`repo_version`) e a última versão mapeada (`ultima_versao_online`) no SQLite local. Ele só dispara uma chamada de rede se o registro estiver pendente de atualização há mais de 24 horas.

---

## 💾 2. Regras e Estados Relacionais (State DB v6)

A máquina de estados de sincronização do SODA ETL (conforme a **ADR-019 v2.0**) governa o ciclo de vida do batedor reativo.

### 2.1 Colunas Envolvidas na Tabela `repo_heuristics`
- `repo_version` (TEXT): O hash curto de commit ou tag local congelado na análise anterior.
- `ultima_versao_online` (TEXT): A última tag de versão recuperada de forma assíncrona do GitHub/Crates.io.
- `status_atualizacao` (TEXT): O status de ciclo de vida longo. Se houver desvio (`repo_version != ultima_versao_online`), o status transiciona para `PENDENTE_FASE_0` para forçar o re-processamento.
- `data_ultima_analise` (INTEGER): Timestamp UNIX Epoch em segundos correspondente ao último run do batedor.

---

## ⚙️ 3. Implementação do Olheiro Assíncrono (Rust)

A arquitetura de rede adota as restrições de higiene bare-metal (ADR-030). A biblioteca `reqwest` é instanciada com a flag `rustls-no-provider` para evitar overheads e vazamentos de memória na RAM.

```rust
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tokio::net::UdpSocket;

/// 1. Verificador Rápido e Não-Bloqueante de Conectividade via UDP
pub async fn is_internet_active() -> bool {
    // Tenta abrir um socket UDP local e conectar temporariamente a um DNS público de alta performance (ex: Cloudflare 1.1.1.1:53)
    // Este método é extremamente veloz (<1ms) e não realiza transferências de pacotes reais na rede.
    let socket = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    // Timeout curto de 200ms para evitar starvation no event loop do Tokio
    let timeout = Duration::from_millis(200);
    tokio::time::timeout(timeout, socket.connect("1.1.1.1:53"))
        .await
        .is_ok()
}

/// 2. O Cronjob de Drift Checker (Fase -1 Background Loop)
pub async fn start_reactive_drift_checker(state_db_tx: tokio::sync::mpsc::Sender<StateDbOp>) {
    let mut timer = interval(Duration::from_secs(3600)); // Verifica a cada hora
    
    loop {
        timer.tick().await;
        
        // 1. Verifica conectividade antes de gastar recursos de I/O
        if !is_internet_active().await {
            eprintln!("[DRIFT_SENTINEL] Sistema offline ou restrito. Olheiro em repouso tático.");
            continue;
        }
        
        // 2. Envia sinal ao StateDbWorker para obter repositórios pendentes ou defasados (> 24h)
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let _ = state_db_tx.send(StateDbOp::GetRepositoriesForDriftCheck { reply: reply_tx }).await;
        
        if let Ok(repositories) = reply_rx.await {
            for repo in repositories {
                // 3. Executa a requisição assíncrona com timeout estrito de 5 segundos
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(5))
                    .build()
                    .unwrap();
                
                // Exemplo de chamada para extrair a última versão do GitHub
                let url = format!("{}/releases/latest", repo.repo_url);
                if let Ok(response) = client.get(&url).send().await {
                    if let Ok(latest_tag) = response.url().path_segments().and_then(|s| s.last()).map(String::from) {
                        // 4. Se a tag online for diferente da local, notifica o banco via MPSC para aplicar a atualização relacional
                        let _ = state_db_tx.send(StateDbOp::UpdateRepositoryDrift {
                            repo_url: repo.repo_url.clone(),
                            online_version: latest_tag,
                        }).await;
                    }
                }
            }
        }
    }
}
```

---

## 🔬 4. Caderno de Testes TDD (DoD GREEN)

Escreveremos e rodaremos os seguintes testes funcionais sob `cargo test --bin souls_mcp_server`:

1. `test_drift_sentinel_offline_bypass`: Mocks de conexões de rede simulando falha de conectividade DNS e garante por meio de asserções que o `is_internet_active` retorna `false` sem lançar panics na thread do Tokio.
2. `test_drift_calculation_and_state_transition`: Popula a tabela `repo_heuristics` com dados de teste (`repo_version = "v1.2.0"`) e insere uma tag online atualizada (`ultima_versao_online = "v1.3.0"`). Assevere que a view de estado ou o rusqlite altera de forma sínclona o `status_atualizacao` para `PENDENTE_FASE_0` e o `status_processamento` de `repositorios` para `PENDENTE`.
3. `test_drift_time_cooldown_gate`: Prova por meio de asserções que repositórios cuja `data_ultima_analise` seja inferior a 24 horas são ignorados pelo olheiro, preservando as cotas da API de rede de forma robusta e otimizada (FinOps de rede).
