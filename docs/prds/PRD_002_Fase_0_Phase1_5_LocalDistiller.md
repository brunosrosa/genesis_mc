# PRD 002 — Phase 1.5 LocalDistiller

## 1. Objetivo Atômico

Implementar o `LocalDistiller` em Rust. O módulo receberá um texto bruto (blob da Zona Amarela), "acordará" a GPU (6GB VRAM), carregará o modelo local Qwen (quantizado em Q4_K_M) e fará a destilação, ejetando um resumo factual denso (a `_essence_`) de aproximadamente 3.000 tokens.

Escopo mecânico deste PRD:

- Cobrir exclusivamente o nó `N4 (LocalDistiller)`.
- Receber um payload de texto (16k-64k tokens) e uma instrução de sistema base.
- Carregar o modelo GGUF quantizado (Qwen3.5-4B-Q4_K_M) em memória GPU.
- Executar inferência sequencial com amostragem determinística.
- Ejetar o texto destilado (3.000 tokens) via `Stream` ou retorno direto.
- Descarregar o modelo e expurgar o KV Cache da VRAM imediatamente via RAII.

## 2. Contrato de I/O (Entrada e Saída)

### Entrada

- `String` contendo o payload do blob (texto bruto de 16k-64k tokens).
- `String` contendo a instrução base de destilação (system prompt).

### Saída

- `Result<String, DistillationError>` contendo o texto destilado puro (a essência factual, max 3.000 tokens).

### Tipos de Erro

```rust
#[derive(Error, Debug)]
pub enum DistillationError {
    #[error("Falha ao carregar modelo GGUF: {0}")]
    ModelLoadError(String),
    #[error("Falha na inferência: {0}")]
    InferenceError(String),
    #[error("Memória GPU insuficiente: {0}")]
    GpuOomError(String),
    #[error("Texto de entrada vazio ou inválido")]
    InvalidInput,
}
```

### Modelo Local

- **Path**: `C:\Users\rosas\.lmstudio\models\lmstudio-community\Qwen3.5-4B-GGUF\Qwen3.5-4B-Q4_K_M.gguf`
- **Quantização**: Q4_K_M (4-bit, ~2.8GB em RAM, ~3.5GB em VRAM)
- **Contexto**: 8k tokens (sufficient para blobs de 16k-64k via chunking)

### Estratégia de Chunking

Para blobs maiores que 8k tokens de contexto:

1. Fragmentar o input em chunks de 6k tokens (overlap de 512 tokens).
2. Inferir cada chunk sequencialmente.
3. Agregar as essências parciais em um único output de ~3.000 tokens.

## 3. Proibições Tóxicas (Red Lines)

### PROIBIDO MANTER O KV CACHE APÓS O USO

O módulo DEVE aplicar a lei do FastSwitch. Imediatamente após a inferência (ejeção da essência), o KV Cache na VRAM DEVE ser expurgado/destruído usando as premissas de RAII (Drop trait) do Rust. A placa de vídeo deve ficar com a memória limpa para o próximo repositório, evitando o letal Spillover da PCIe.

Mecanismo obrigatório:

```rust
impl Drop for LocalDistiller {
    fn drop(&mut self) {
        // 1. Descarregar modelo da GPU
        self.model.clear_cache();
        // 2. Forçar sincronização
        self.device.synchronize();
        // 3. Log da limpeza para auditoria
        tracing::info!("KV Cache expurgado, VRAM limpa");
    }
}
```

### PROIBIDA COMPLEXIDADE DESNECESSÁRIA DE KERNELS

A inferência deve se basear em bindings locais robustos e diretos para o hardware (como `candle-core` ou `llama-cpp-4`) focados na estabilidade bare-metal, sem exigir recompilações JIT no meio da execução.

Bibliotecas recomendadas (em ordem de preferência):

1. **`candle-core`** (Burn/Native Rust, sem FFI externo)
2. **`llama-cpp-4`** (若找不到 candle, usar GGUF bindings via `llama-cpp-4` ou `candle-gguf`)

## 4. Definition of Done (DoD) & TDD

### Teste Unitário: Mock do Motor de Inferência

- Criar teste `#[test]` com um "dummy text" de 20.000 tokens simulados.
- O teste deve verificar que o output respeita o teto de 3.000 tokens.
- O teste DEVE usar um mock do motor de inferência (não requer GPU no CI).

### Teste de Integração: Carga Real (CI Opcional)

- Teste de carga com arquivo real de 30k tokens.
- Verificar que o output está no range de 2.800-3.200 tokens.
- Prova de descarregamento de memória via verificação de `device.allocated_memory() == 0`.

### Prova de Descarregamento de Memória

```rust
#[test]
fn test_vram_cleaned_after_inference() {
    let before = device.allocated_memory();
    let distiller = LocalDistiller::new(model_path).unwrap();
    drop(distiller);
    let after = device.allocated_memory();
    assert_eq!(after, 0, "VRAM deve estar limpa após Drop");
}
```

### Critérios de Aceitação

- Módulo passa em `cargo clippy -- -D warnings`.
- Nenhum panicunwrapexpect em produção (apenas em testes).
- O KV Cache é provado limpo após cada inferência.
- Output controlado a 3.000 tokens (±10%).
