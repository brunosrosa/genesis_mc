---
id: "ADR-035"
title: "ADR-035-Reparo-Sintatico-Zero-Token-via-IPC"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura"
description: "Impõe a interceptação do buffer IPC de saída do LLM por um motor Rust (jsonrepair) para cura sintática zero-token em sub-milissegundo."
---

# ADR-035: Reparo Sintático Zero-Token via IPC

## Status

Aceito (Ativo, Inegociável e Fundacional para SOULS V5)

## Contexto Técnico e Desperdício de Tokens

Mesmo quando a amostragem de tokens é governada por decodificação restrita (`llguidance`), Small Language Models (SLMs) operando em hardware restrito estão sujeitos a estouros de limite de contexto ou interrupções abruptas que geram payloads malformados. Em arquiteturas agênticas tradicionais, um erro de parse sintático (como um JSON com chaves não fechadas ou cercas Markdown indesejadas) resulta em duas abordagens ineficientes:

1. **Re-prefill e Re-geração Completa:** Reenviar o prompt corrigido ao modelo, gerando novo consumo de tokens, desperdício de energia térmica na GPU e latência adicional de $1.5\text{s}$ a $4.0\text{s}$.
2. **Falha Fatal e Cancelamento:** Reportar erro imediato ao pipeline agêntico, interrompendo a cadeia de execução do usuário.

## Declaração do Problema

Como sanar 99% das falhas sintáticas em respostas de SLMs (JSONs truncados, falta de aspas, vírgulas no último elemento, blocos Markdown) com latência sub-milissegundo e **custo zero de tokens e VRAM**, eliminando re-prefills desnecessários na GPU?

## Decisões Arquiteturais da SOULS V5

```
                       [ BUFFER IPC STREAMING (GPU/LLM) ]
                                       |
                                       v
                     [ Parser Estrito (serde_json) ]
                                       |
                     +-----------------+-----------------+
                     |                                   |
              (Sucesso / Valido)                  (Falha Sintática)
                     |                                   |
                     v                                   v
             [ Payload Aceito ]           [ Motor Rust: jsonrepair ]
                                          (&str Slicing Zero-Copy < 1ms)
                                                         |
                                                         v
                                              [ Payload Curado & RFC 8259 ]
                                                         |
                                                         v
                                              [ Custo VRAM = 0 MB ]
                                              [ Tokens Extra = 0 ]
```

### 1. Interceptação Obrigatória no Buffer IPC de Saída

Fica **OBRIGATÓRIO** que toda a resposta emitida pelo motor de inferência (`llama.cpp` / `mistral.rs`) seja interceptada na camada de transporte IPC do Gateway Rust antes de ser exposta ao executor do agente ou ao cliente de interface.

- Se a desserialização via parser estrito (`serde_json`) falhar, o Gateway DEVE submeter o buffer bruto ao pipeline nativo de **Response Healing** em Rust.
- É terminantemente **PROIBIDO** disparar uma nova requisição de geração à GPU sem antes esgotar a tentativa de reparo sintático no Host.

### 2. Stack de Reparo Zero-Token em Rust (`jsonrepair` / `llm_json`)

O motor de cura sintática opera via crates compilados nativamente em Rust (ex: `jsonrepair`, `llm_json`, `fast_json_repair`) aplicando transformações baseadas em autômatos finitos e *zero-copy slicing* (`&str`):

1. **Sanitização de Cercas Markdown:** Stripping síncrono de tags de bloco (ex: ` ```json ... ``` `) e fragmentos de texto conversacional anexados nas bordas do payload.
2. **Correção de Delimitadores e Aspas:** Conversão de aspas simples para duplas, ajuste de aspas não escapadas e saneamento de identificadores de chave desaspados.
3. **Remoção de Trailing Commas & Inserção de Vírgulas Múltiplas:** Saneamento de listas e dicionários com vírgulas sobressalentes ou ausentes entre elementos.
4. **Fechamento por Pilha de Delimitadores (Truncamento):** Em fluxos interrompidos por limite de tokens, o motor varre a pilha de estados e injeta automaticamente os delimitadores de fechamento (`}` e `]`) necessários para validar o trecho parcial de dados.
5. **Coerção de Literais:** Normalização instantânea de primitivas Python/JS (`True`, `False`, `None`, `undefined`) para a especificação JSON RFC 8259 (`true`, `false`, `null`).

### 3. Latência e Custo Termodinâmico

- **Custo de VRAM:** $0\text{ MB}$ (processamento $100\%$ executado na CPU do Host).
- **Orçamento de Latência:** O pipeline de *Response Healing* em Rust DEVE ser concluído em **menos de $1,0 \text{ ms}$** para payloads de até $64\text{ KB}$.
- **Métrica de Desempenho:** A taxa de resgate sintático zero-token DEVE atingir $\ge 95\%$ das falhas de formatação capturadas em ambiente de produção.

## Consequências e Trade-offs

### Impactos Positivos:

- **Redução Drástica de Re-prefills:** Elimina o desperdício de VRAM e energia térmica decorrentes de re-gerações provocadas por erros sintáticos triviais.
- **Robustez Agêntica:** O agente mantém a fluidez operacional mesmo quando a saída do SLM 4B é cortada no limite da janela de contexto.
- **Transparência Total:** A cura sintática é invisível para a camada superior do sistema, registrando estatísticas de resgate na telemetria sem atrasar a UX.

### Impactos Negativos:

- **Risco de Resgate de Dados Incompletos:** Se a resposta for truncada no meio de um valor numérico ou string crítica, o auto-fechamento do JSON salvará a estrutura parcial, exigindo validação de esquema de negócio na camada agêntica posterior.

### Comportamento Fail-Closed

Se o motor `jsonrepair` em Rust não conseguir produzir um JSON válido e tipado após a passagem heurística, o evento é classificado como falha estrutural insuperável. Nesse caso específico, a requisição é direcionada ao **Ralph Loop Sequencial** (ADR-036) para tratamento com traço de erro explícito.
