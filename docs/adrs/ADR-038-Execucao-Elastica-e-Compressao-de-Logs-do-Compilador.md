---
id: "ADR-038"
title: "ADR-038: Execução Elástica e Compressão de Logs do Compilador"
version: 1.0
status: Proposed
epic: "Infraestrutura"
description: "Isolamento de Stdio em execuções de terminal assíncronas do Tokio e compressão de logs de compilador via Pattern Log Compression para mitigação de Context Rot e sanidade do MCP JSON-RPC."
---

# ADR-038: Execução Elástica e Compressão de Logs do Compilador

## Status

Proposed (Proposto para Cânone SOULS V4)

## Contexto Técnico e Ameaça ao Protocolo MCP

Durante os ciclos de desenvolvimento iterativo e loops de autonomia agêntica (TDD / Ralph Loop), os subagentes executam frequentemente comandos pesados de compilação, verificação estática ou execução de suítes de testes (`cargo check`, `cargo clippy`, `cargo test`). 

A emissão descontrolada e bruta dessas saídas de terminal traz duas ameaças críticas ao sistema:

1. **Saturação da Janela de Atenção ("Context Rot"):** Enviar milhares de linhas de logs brutos de compilação — incluindo warnings cosméticos de linter, mensagens repetitivas de compilação de crates intermediárias e relatórios verbosos — destrói o orçamento de tokens do LLM e degrada severamente o alinhamento semântico do modelo.
2. **Corrupção Fatal do Protocolo MCP (JSON-RPC Breakdown):** Se o processo hospedeiro ou sidecars emitirem saídas textuais ou warnings diretamente na saída padrão (`stdout`) do processo principal, as mensagens do barramento JSON-RPC do protocolo MCP serão corrompidas (`invalid Content-Length` ou JSON inválido), provocando a queda instantânea da sessão e da IDE.

## Decisões Inegociáveis

### 1. Isolamento Absoluto de Standard Output em Execuções de Terminal

- Toda e qualquer execução de comandos de terminal engatilhada por agentes (por exemplo, via `souls_shell`) deve ser instanciada estritamente através do executor assíncrono do Tokio (`tokio::process::Command`).
- Os pipes de saída padrão (`stdout`) e erro padrão (`stderr`) do subprocesso trabalhador devem ser compulsoriamente redirecionados para `Stdio::piped()`.
- Os bytes emitidos pelo subprocesso são capturados de forma totalmente privada em buffers de memória no host Rust, sendo **TERMINANTEMENTE PROIBIDO** qualquer vazamento direto para o `stdout` do processo pai. Isso garante imunidade total contra a corrupção de enquadramento das mensagens JSON-RPC do canal MCP.

### 2. Algoritmo de Pattern Log Compression (Poda de 90%)

- O motor Rust do SOULS intercepta o fluxo textual bruto capturado dos buffers privados do subprocesso e aplica a pipeline de **Pattern Log Compression**.
- O compressor filtra e elimina até 90% do volume textual inútil, expurgando:
  - Warnings cosméticos de linter desnecessários para a resolução da falha imediata.
  - Linhas repetitivas de progresso de compilação de crates de terceiros (`Compiling ...`, `Downloaded ...`).
  - Cabulagens e informações verbosas de sucesso intermediário.
- O algoritmo reconstrói e sintetiza um relatório ultracompacto focado, entregando ao contexto do LLM estritamente:
  - As linhas exatas com erros sintáticos estruturados (caminho do arquivo, linha, coluna e mensagem de erro do compilador).
  - Stacktraces nativos de panic em falhas de execução.
  - Asserções e relatórios exatos de falhas de testes comportamentais.

## Consequências e Trade-offs

### Impactos Positivos:
- **Sanidade do Barramento MCP:** Imunidade absoluta contra desconexões acidentais da sessão MCP por contaminação de `stdout`.
- **Economia Extrema de Contexto:** Economia de até 90% dos tokens gastos com saídas de terminal durante loops de TDD/Ralph Loop.
- **Raciocínio Cirúrgico:** Agentes focam imediatamente na causa raiz do erro sintático ou falha de teste sem ruído de linter.

### Impactos Negativos:
- **Perda de Verbosidade Cosmética:** Em diagnósticos complexos de build onde um warning omitido pode indicar a causa raiz indireta, o operador humano pode precisar requisitar o log bruto não comprimido.
