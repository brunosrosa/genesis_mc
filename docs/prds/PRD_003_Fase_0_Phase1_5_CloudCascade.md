# PRD 003 — Phase 1.5 Cloud Cascade Router

## 1. Objetivo Atômico

Implementar o `CloudCascade` em Rust (usando `reqwest` nativo). O módulo receberá um blob denso (> 64k tokens) roteado para a Zona Vermelha e fará a destilação em nuvem para ejetar uma `_essence_` de ~3.000 tokens, otimizando o custo via Fallback Automático.

Escopo mecânico deste PRD:

- Cobrir exclusivamente os nós `N6 (CloudFreeCascade)` e `N7 (CloudPaidFallback)`.
- Receber um payload de texto (> 64k tokens) e uma instrução de sistema.
- Executar destilação via OpenRouter com modelo gratuito como primeira tentativa.
- Implementar fallback automático para modelo pago apenas quando HTTP 429/503 for detectado.
- Retornar a essência destilada ou um erro terminal.

## 2. Contrato de I/O (Entrada e Saída)

### Entrada

- `String` contendo o payload massivo (> 64k tokens).
- `String` contendo a instrução base de destilação (system prompt).

### Saída

- `Result<String, CascadeError>` contendo o texto destilado puro (~3.000 tokens).

### Tipos de Erro

```rust
#[derive(Error, Debug, Clone)]
pub enum CascadeError {
    #[error("Payload invalido ou vazio")]
    InvalidInput,
    #[error("Modelo gratuito indisponivel (HTTP {status})")]
    FreeTierUnavailable { status: u16 },
    #[error("Modelo pago falhou (HTTP {status}): {message}")]
    PaidFallbackFailed { status: u16, message: String },
    #[error("Timeout na requisicao: {0}")]
    RequestTimeout(String),
    #[error("Erro de rede: {0}")]
    NetworkError(String),
}
```

### Modelo Gratuito (N6 — CloudFreeCascade)

- **Provider**: OpenRouter
- **Modelo**: `qwen/qwen3-coder:free`
- **Endpoint**: `POST https://openrouter.ai/api/v1/chat/completions`
- **Custo**: $0.00 por token

### Modelo Pago (N7 — CloudPaidFallback)

- **Provider**: OpenRouter
- **Modelo**: `deepseek/deepseek-v4-flash`
- **Endpoint**: `POST https://openrouter.ai/api/v1/chat/completions`
- **Custo**: ~$0.10 por 1M tokens (Flash)

## 3. A Lógica da Cascata (FinOps)

### Diagrama de Estados

```
[Payload > 64k]
       │
       ▼
┌──────────────────┐
│ CloudFreeCascade  │
│ qwen/qwen3-coder │
└────────┬─────────┘
         │
    ┌────┴────┐
    │ HTTP    │
    │ Response │
    └────┬────┘
         │
    ┌────┴────────────────┐
    │ 200 OK?             │
    └─────────┬────────────┘
         YES │           NO
             │    ┌───────────────┐
             │    │ Rate Limit    │
             │    │ 429 ou 503?   │
             │    └───────┬───────┘
             │        YES │
             │            ▼
             │   ┌──────────────────┐
             │   │ CloudPaidFallback │
             │   │ deepseek-v4-flash│
             │   └────────┬─────────┘
             │            │
             │       ┌────┴────┐
             │       │ 200 OK? │
             │       └────┬────┘
             │        YES │    NO
             │            │    │
             ▼            ▼    ▼
        [essence]    [essence] [CascadeError]
```

### Implementação

```rust
pub async fn cascade_distill(
    &self,
    payload: &str,
    system_prompt: &str,
) -> Result<String, CascadeError> {
    // Passo 1: Tentativa Gratuita
    let result = self.call_openrouter(
        payload,
        system_prompt,
        "qwen/qwen3-coder:free",
    ).await;

    match result {
        Ok(essence) => return Ok(essence),
        Err(CascadeError::FreeTierUnavailable { status }) if status == 429 || status == 503 => {
            // Passo 2: Fallback para Pago
            tracing::info!("CloudCascade: Free tier unavailable ({}), switching to paid", status);
            self.call_openrouter(
                payload,
                system_prompt,
                "deepseek/deepseek-v4-flash",
            ).await
        }
        Err(e) => return Err(e),
    }
}
```

## 4. Proibições Tóxicas (Red Lines)

### PROIBIDO LOGAR O PAYLOAD

Como o input possui mais de 64k tokens, é terminantemente PROIBIDO imprimir o `payload` bruto ou o `prompt` em `tracing::info!` ou `println!`. O log causaria asfixia de I/O no terminal. Apenas as decisões de rota e metadados devem ser logados.

```rust
// CORRETO: Log apenas de metadados
tracing::info!(
    "CloudCascade: payload_tokens={}, route=Free, status=200"
);

// INCORRETO: Proibido
tracing::info!("Payload: {}", payload); // ← VAZA MEMÓRIA DE I/O
```

### PROIBIDO RETENTATIVAS INFINITAS

O fallback ocorre apenas uma vez (Free -> Paid). Se o Paid falhar, retorne um `CascadeError`. Nada de loops cegos de retry.

```rust
// CORRETO: Uma única retentativa
match self.cascade_distill(payload, prompt).await {
    Ok(essence) => Ok(essence),
    Err(e) => Err(e), // Sem loop de retry
}

// INCORRETO: Loop infinito
loop {
    let result = self.call_openrouter(...).await;
    if result.is_ok() { break; } // ← PROIBIDO
}
```

## 5. Definition of Done (DoD) & TDD

### Teste 1: Free Tier Sucesso (200 OK)

- Mock do servidor OpenRouter retornando HTTP 200 com JSON válido.
- Verificar que `cascade_distill` retorna `Ok(essence)`.
- Verificar que o modelo usado foi `qwen/qwen3-coder:free`.

### Teste 2: Free Tier Rate Limit + Fallback para Paid

- Mock do servidor OpenRouter:
  - Primeira chamada: HTTP 429 (Rate Limit).
  - Segunda chamada: HTTP 200 com JSON válido.
- Verificar que `cascade_distill` retorna `Ok(essence)`.
- Verificar que houve exatamente 2 chamadas HTTP (Free + Paid).

### Teste 3: Paid Tier Também Falha

- Mock do servidor OpenRouter:
  - Primeira chamada: HTTP 429.
  - Segunda chamada: HTTP 500 (Erro interno).
- Verificar que `cascade_distill` retorna `Err(CascadeError::PaidFallbackFailed { ... })`.
- Verificar que houve exatamente 2 chamadas HTTP (sem loop).

### Teste 4: Input Inválido

- Passar payload vazio ou whitespace.
- Verificar que retorna `Err(CascadeError::InvalidInput)`.

### Critérios de Aceitação

- Módulo passa em `cargo clippy -- -D warnings`.
- Nenhum `unwrap()` ou `expect()` em código de produção (apenas testes).
- Payload nunca é logado, apenas metadados (token count, modelo, status HTTP).
- Fallback ocorre no máximo 1 vez.

### Dependências de Mock

- `mockito` (já presente em `[dev-dependencies]`) para simular servidor HTTP.
