---
title: "Design: Válvula FinOps (Cloud Bypass)"
status: "ativo"
trigger: "feat/finops-cloud-bypass"
---

## 1. Contexto e Objetivo

Implementar a "Válvula de Nuvem" (Cloud Bypass) no `ParetoBanditRouter` (N3 - FinOpsRouter).
Durante a Fase 1.5 (Lote 01), o hardware local (RTX 2060m) deve ser poupado, desviando
todos os blobs da Zona Amarela (16k-64k tokens) para o `CloudCascade`.

### Zona de Risco FinOps

| Zona | Tokens | Comportamento Padrão | Comportamento Factory |
|------|--------|---------------------|----------------------|
| Green | < 16k | Pass-Through | Inalterado |
| Yellow | 16k-64k | LocalModel (Qwen) | CloudCascade |
| Red | > 64k | CloudCascade | Inalterado |

---

## 2. Contrato de I/O

### Entrada

- `PathBuf` para o arquivo blob (igual ao existente)
- Variável de ambiente `SODA_FACTORY_CLOUD_ONLY`

### Saída

- `RoutingDecision` com `destination` modificado para Yellow quando bypass ativo

### Regras de Fail-Closed

- Se `SODA_FACTORY_CLOUD_ONLY` não estiver definida → comportamento padrão (Local-First)
- Se `SODA_FACTORY_CLOUD_ONLY` = `"true"` ou `"1"` → bypass ativado
- Qualquer outro valor → comportamento padrão

---

## 3. Arquitetura da Mutação

### Arquivo: `src-tauri/src/finops/finops_router.rs`

```rust
// Adicionar função helper
fn is_factory_cloud_only() -> bool {
    match std::env::var("SODA_FACTORY_CLOUD_ONLY") {
        Ok(val) => val.eq_ignore_ascii_case("true") || val == "1",
        Err(_) => false,
    }
}

// Modificar classify_blob para verificar bypass
// Yellow Zone: se bypass ativo → CloudCascade, senão → LocalModel
```

### Testes TDD (RED primeiro)

1. `test_30k_yellow_without_bypass_routes_to_local` - Sem var, 30k → LocalModel
2. `test_30k_yellow_with_bypass_routes_to_cloud` - Com var=true, 30k → CloudCascade
3. `test_70k_red_ignores_bypass` - Com var=true, 70k → CloudCascade (inalterado)
4. `test_10k_green_ignores_bypass` - Com var=true, 10k → PassThrough (inalterado)

---

## 4. Red Lines (Inegociáveis)

- PROIBIDO deletar ou comentar código do `LocalDistiller`
- PROIBIDO alterar lógica de Green (<16k) e Red (>64k)
- Fail-Closed: qualquer erro de leitura da env var → default Local-First

---

## 5. Definição de Pronto (DoD)

- [ ] `cargo test` passa com 4 novos testes
- [ ] `cargo clippy -- -D warnings` sem warnings
- [ ] Zero mutações em `orchestrator.rs` ou `local_distiller.rs`
