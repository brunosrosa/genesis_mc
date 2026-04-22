---
name: soda-ralph-loop
description: O Motor de Resiliência e Autocorreção do SODA. Impõe a execução do Ralph Loop via scripts de terminal. Aplica 'cargo check' em JSON, comprime o erro via Toon Context e roda até obter Exit Code 0 ou atingir o Kill-Switch.
triggers: ["soda-ralph-loop", "testar código", "rodar testes", "corrigir erro", "loop de compilação", "ralph loop", "auto-fix", "debug"]
---

# Skill: SODA Ralph Loop (O Motor Implacável de Resiliência)

## Goal
Atuar como a esteira de Integração Contínua (CI) e autocorreção local do Genesis MC. O objetivo inegociável desta habilidade é blindar a energia cognitiva do usuário (2e/TDAH) contra a exaustão de depurar erros de sintaxe ou de compilação no terminal. Você operará sob o paradigma do "Determinismo Destrutivo", utilizando scripts executáveis em *background* para testar, falhar, expurgar o seu contexto viciado (Context Rot) e tentar novamente até que a máquina física ateste perfeição matemática (Exit Code 0).

## Instructions (Execução Coercitiva)
Sempre que finalizar a escrita de um bloco de código, você ESTÁ PROIBIDO de entregá-lo passivamente ao usuário dizendo "Aqui está o código, por favor teste". Você DEVE testá-lo autonomamente executando a seguinte Máquina de Estados:

1. **A Invocação do Executor (O Script de Nível 3):**
   - Não rode comandos como `cargo check` soltos no terminal interativo.
   - Você DEVE invocar a caixa-preta executável localizada em `scripts/ralph_executor.sh`. Este script gerencia o loop de *Backpressure* não-determinístico isolando as falhas do seu contexto principal.

2. **Destilação Estrita de Erros (Invocação Toon Context):**
   - O script executará a validação estrutural do Rust usando `cargo check --message-format=json` (ou linter/testador equivalente para outras linguagens).
   - Se houver falha, você NUNCA deve tentar ler o log bruto do terminal carregado de ruídos ANSI e "ASCII art". 
   - Você DEVE repassar o output JSON de erro para o servidor MCP `Toon Context`. Ele minificará a resposta e extrairá exclusivamente o identificador do erro (ex: E0119), as linhas ofensivas exatas e as prescrições de sintaxe. Leia APENAS esta documentação destilada.

3. **Morte e Renascimento (Obligatory Context Purge):**
   - Esqueça o seu raciocínio gerativo anterior que levou à falha; ele está corrompido.
   - Com base no log purificado pelo Toon Context, aplique a mutação no arquivo de código e inicie a iteração de teste novamente de forma limpa e isolada.

4. **O Kill-Switch (Limite de Iterações e Circuit Breaker):**
   - O script `ralph_executor.sh` possui um limite rígido de tentativas travado em `maxIterations=3`. 
   - Se o script abortar informando que o limite foi atingido (indicando uma "espiral recursiva" de alucinação), o seu loop interno MORRE imediatamente. 
   - Ao atingir o Kill-Switch, gere um sumário técnico conciso no Canvas (Diagnóstico, Passo a Passo de Investigação) listando por que o compilador não aprova a rota atual, e requisite a intuição criativa e a intervenção humana (HITL) do Arquiteto.

5. **A Entrega Silenciosa (Exit Code 0):**
   - Se o código compilar com sucesso (*Exit Code 0*), você encerra o loop de fundo.
   - Notifique o usuário na interface do Canvas de forma espartana que a tarefa foi implementada e duplamente validada pela física do compilador.

## Constraints
* **FIM DO VIBE CODING REVISIONAL:** O compilador (rustc/tsc) é a única fonte da verdade. Não adivinhe soluções; analise friamente a saída sintática destilada.
* **SILÊNCIO OPERACIONAL:** O usuário não deve ser incomodado com notificações a cada tentativa falha. O processo de "tentativa e erro" é o trabalho sujo do *background*. Exiba apenas o sucesso terminal ou o fracasso fatal.

## Examples
**Entrada do Usuário:** 
"A lógica de roteamento do SQLite tá pronta. Roda o Ralph Loop pra garantir que passa no Borrow Checker do Rust."

**Ação do Agente:**
1. O agente invoca o script `ralph_executor.sh`, que roda `cargo check --message-format=json`.
2. O terminal falha. O erro bruto é passado ao Toon Context, que devolve de forma limpa: `error[E0507]: cannot move out of 'state' which is behind a shared reference`.
3. O agente destrói a justificativa anterior, foca no erro de Ownership, altera a variável para usar `Arc<Mutex<State>>` e o executor roda novamente.
4. Passa com Exit Code 0.
5. Agente responde na UI: *"Ralph Loop concluído. Compilação em Rust bem-sucedida (Exit Code 0). O estado compartilhado foi envelopado em Mutex. Aguardando próximas diretrizes."*