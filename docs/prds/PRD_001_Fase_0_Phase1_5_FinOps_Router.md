# PRD 001 — Phase 1.5 FinOps Router

## 1. Objetivo Atômico

Implementar o disjuntor FinOps em Rust. O módulo deve ler o caminho de um
`_blob_bruto`, usar a biblioteca `tiktoken-rs` localmente para contar os tokens
em `O(1)` e retornar uma decisão de roteamento baseada nos limites de zona.

Escopo mecânico deste PRD:

- Cobrir exclusivamente os nós `N2 (PreFlightTokenizer)` e `N3 (ParetoBanditRouter)`.
- Medir um blob por vez, a partir de um arquivo físico já persistido no disco.
- Transformar a contagem exata de tokens em um plano de voo determinístico.
- Encerrar a responsabilidade no retorno da decisão; nenhuma execução de worker
  local ou chamada em nuvem ocorre aqui.

## 2. Contrato de I/O (Entrada e Saída)

### Entrada

- `PathBuf` apontando para o arquivo físico do `_blob_`.

### Saída

- Um `struct` ou `enum` de decisão, por exemplo `RoutingDecision`, contendo:
  - `token_count` exato calculado localmente via `tiktoken-rs`.
  - `RoutingZone` com uma das três zonas: `Green`, `Yellow`, `Red`.
  - O destino operacional correspondente:
    - `Green`: `Pass-Through`
    - `Yellow`: `C:\Users\rosas\.lmstudio\models\lmstudio-community\Qwen3.5-4B-GGUF\Qwen3.5-4B-Q4_K_M.gguf`
    - `Red`: plano de cascata em nuvem via OpenRouter

### Regra Matemática de Classificação

- `Green`: blob com menos de `16k tokens`
- `Yellow`: blob entre `16k` e `64k tokens`
- `Red`: blob com mais de `64k tokens`

### Invariantes do Contrato

- A contagem deve refletir o conteúdo real do arquivo apontado pelo `PathBuf`.
- A resposta deve ser determinística: mesmo arquivo, mesma contagem, mesma zona.
- O módulo deve devolver metadado suficiente para o estágio seguinte executar a rota,
  mas não deve executar a rota.

## 3. Proibições Tóxicas (Red Lines)

- **PROIBIDO CARREGAR EM MASSA:** o módulo deve processar um blob por vez,
  sequencialmente. Ler os 11 blobs simultaneamente para a RAM para contar tokens
  viola a regra de segurança contra OOM.
- **PROIBIDO I/O DE REDE NESTE MÓDULO:** este PRD trata APENAS do disjuntor,
  isto é, a contagem e a matemática da decisão. Ele NÃO deve executar chamadas
  LLM nem invocar `Qwen` ou `DeepSeek`. Ele apenas devolve o plano de voo.

## 4. Definition of Done (DoD) & TDD

- Criar testes unitários com `#[test]` cobrindo as fronteiras matemáticas do roteador.
- Provar que um arquivo simulado de `10k tokens` retorna `Zona Verde`.
- Provar que um arquivo simulado de `30k tokens` retorna `Zona Amarela` com o modelo:
  `C:\Users\rosas\.lmstudio\models\lmstudio-community\Qwen3.5-4B-GGUF\Qwen3.5-4B-Q4_K_M.gguf`
- Provar que um arquivo simulado de `70k tokens` retorna `Zona Vermelha`
  com decisão de `Cloud Cascading`.
- Garantir que o módulo passe em `cargo clippy -- -D warnings`.
- Não escrever nenhuma lógica de execução de modelos neste PRD; o sucesso é a
  decisão correta, não a inferência.
