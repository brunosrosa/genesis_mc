# PRD-005: Vacina Contra Memory Bloat — Faxina e Evicção de Cache do `lean_vacuum`

Este documento especifica a implementação da "vacina" de gerenciamento de ciclo de vida do cache em RAM, projetada para evitar vazamentos de memória e inchaço de recursos (Memory Bloat) no daemon central do Souls MC sob execução contínua 24/7.

---

## 1. DECLARAÇÃO DO PROBLEMA E REQUISITOS OPERACIONAIS

A nova implementação do `souls_dedup` introduziu o `SESSION_DEDUP_CACHE` em RAM (via `DashMap` concorrente global). Como o daemon do SOULS opera de forma persistente em segundo plano no Host, a varredura contínua de múltiplos arquivos e sessões de chat acumula hashes de 64 bits de forma monotônica. 

Sem uma rotina explícita de limpeza e evicção, o consumo de RAM do Host crescerá linearmente ao longo do tempo, violando os limites termodinâmicos estipulados na Matriz Fundacional do SOULS V4.

### Requisitos Inegociáveis:
1. **Evicção Determinística:** O sistema deve expor uma interface pública segura em Rust para limpar o cache em tempo constante $\mathcal{O}(1)$.
2. **Integração com Sessão (MCP):** A limpeza deve ser exposta como uma ferramenta MCP dedicada ou acoplada às rotinas de ciclo de vida do barramento de controle (`souls_session`).
3. **Garantia de Isolamento de Concorrência:** O reset de estado deve ocorrer de forma thread-safe utilizando as garantias nativas de concorrência do `DashMap` e `std::sync::LazyLock`, sem causar panics ou travar leituras paralelas ativas.

---

## 2. ESPECIFICAÇÃO DE MUTAÇÃO E PROJETO DE CÓDIGO

### A. Exposição da Interface de Limpeza em `dedup.rs`
O módulo `src-tauri/src/cognition/lean_vacuum/dedup.rs` deve expor uma função pública de descarte:

```rust
/// Limpa completamente o cache de deduplicação da sessão em RAM.
/// Invoca o descarte físico dos nós para liberar a memória principal do Host.
pub fn clear_session_cache() {
    SESSION_DEDUP_CACHE.clear();
}
```

### B. Integração do Gateway MCP (`souls_mcp_server.rs`)
Se o gateway MCP expuser a ferramenta `souls_session` (ou se for necessário criá-la), ela deve processar a ação `"clear"` ou `"reset"`.

Caso a ferramenta `souls_session` ainda não esteja totalmente conectada, podemos registrar opcionalmente o comando `"souls_session"` ou estender o gateway para expor o descarte físico:

```rust
async fn run_souls_session(params: &serde_json::Map<String, Value>) -> Result<Value, RpcError> {
    let args = params.get("arguments").and_then(Value::as_object).unwrap_or(params);
    let action = args.get("action").and_then(Value::as_str).unwrap_or("status");

    match action {
        "clear" | "reset" => {
            // Executa a vacinação limpando o cache na RAM
            lean_vacuum::dedup::clear_session_cache();
            Ok(json!({
                "content": [{
                    "type": "text",
                    "text": "Cache de deduplicação de sessão (lean_vacuum) limpo com sucesso. RAM desidratada."
                }]
            }))
        }
        _ => Err(RpcError {
            code: -32003,
            message: format!("Ação de sessão '{action}' não suportada ou não implementada."),
            data: None,
        })
    }
}
```

---

## 3. SUÍTE DE TESTES UNITÁRIOS REQUERIDA (TDD)

A IDE deve programar e fazer passar o seguinte caso de teste de integridade na suíte do Rust:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_cache_clear_successful() {
        // Passo 1: Garantir que o cache pode receber dados
        let path = std::path::PathBuf::from("src/main.rs");
        
        // Simula preenchimento inserindo dados fictícios no cache
        SESSION_DEDUP_CACHE.insert(12345, (path.clone(), 1, 5));
        assert!(!SESSION_DEDUP_CACHE.is_empty(), "O cache deveria conter dados simulados.");

        // Passo 2: Dispara a vacina de limpeza
        clear_session_cache();

        // Passo 3: Asserção de integridade física da RAM
        assert!(SESSION_DEDUP_CACHE.is_empty(), "O cache deveria estar completamente vazio pós-limpeza.");
    }
}
```

---

## 4. VERIFICATION PLAN

### Testes Automatizados
- Executar os testes unitários:
  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml --lib cognition::lean_vacuum::dedup
  ```
- Validar lints de compilador:
  ```powershell
  cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
  ```
