---
name: mcp-sequential-thinking
description: O freio de mão cognitivo do SODA. Força a IA a aplicar um pipeline estruturado de raciocínio passo a passo via MCP antes de gerar código, evitando a "ansiedade agêntica" e o Vibe Coding.
triggers: ["mcp-sequential-thinking", "raciocinar passo a passo", "pensar", "analisar problema", "planejar refatoração", "sequential thinking", "desdobrar lógica"]
---

# Skill: MCP Sequential Thinking (O Freio de Mão Cognitivo)

## Goal
Atuar como o regulador de cadência cognitiva do agente dentro do Antigravity IDE. O objetivo inegociável desta habilidade é impedir que o LLM tente cuspir soluções colossais, refatorações de código ou arquiteturas complexas em um único fôlego. Para proteger a VRAM local e garantir a exatidão em linguagens estritas como Rust, o agente deve delegar o "Overthinking" para o servidor MCP `sequential-thinking`, desdobrando a lógica emaranhada em subetapas encadeadas, dinâmicas e revisáveis antes de apresentar qualquer código ao usuário.

## Instructions
Sempre que você se deparar com uma arquitetura nova, um bug complexo no compilador, ou quando o usuário pedir uma refatoração profunda, você DEVE acionar o seu freio lógico executando os passos abaixo:

1. **Interdição de Resposta Imediata:** Você está expressamente PROIBIDO de gerar blocos de código (`.rs`, `.tsx`, etc.) na sua primeira resposta.
2. **Invocação do Motor de Pensamento:** Acione as ferramentas fornecidas pelo servidor MCP `sequential-thinking` (declarado no seu `mcp_config.json`).
3. **O Pipeline de 5 Fases:** Submeta o problema ao fluxo iterativo obrigatório da ferramenta:
   - *Problem Definition (Definição):* Qual é a raiz exata do problema?
   - *Research (Pesquisa):* Quais invariantes da arquitetura SODA se aplicam aqui?
   - *Analysis (Análise):* Quais são os gargalos e riscos físicos (ex: OOM, Borrow Checker)?
   - *Synthesis (Síntese):* Qual é a rota de solução mais enxuta?
   - *Conclusion (Conclusão):* A decisão arquitetural final.
4. **Correção de Curso (Self-Correction):** Utilize a capacidade do MCP para revisar ou reverter subetapas do pensamento caso a sua Análise demonstre que a sua Pesquisa ou Definição inicial estava falha.
5. **Emissão Limpa:** Após o MCP retornar a Conclusão validada, você pode, e somente então, acionar a skill `@soda-sdd` ou iniciar a entrega da síntese final para o usuário.

## Constraints
* **FIM DA ANSIEDADE AGÊNTICA:** Não tente resolver tudo mentalmente na sua janela de contexto padrão. Use a ferramenta para externalizar o raciocínio.
* **SILÊNCIO TÁTICO:** O usuário não precisa (nem quer) ler todo o seu log de pensamentos falhos. Utilize o MCP no backend, processe o ruído invisivelmente, e exiba no Canvas/Chat apenas o *Output* da "Conclusion" limpa.
* **PROIBIÇÃO DE VIBE CODING:** Pular a invocação desta ferramenta em tarefas de engenharia de software resultará em código frágil e será considerado uma violação crítica do protocolo SODA.

## Examples

**Entrada do Usuário:** 
"SODA, precisamos reescrever o roteador de LLMs do AgentGateway. O código em Python antigo dava *Event Loop Starvation*. Faz isso em Rust agora usando Tokio MPSC."

**Ação Incorreta (NÃO FAÇA):**
O agente diz "Claro!" e imediatamente cospe 300 linhas de código Rust usando `std::sync::Mutex`, ignorando o bloqueio assíncrono e falhando miseravelmente.

**Ação Correta (Obrigatória):**
1. O agente recusa a codificação imediata.
2. Invoca o MCP `sequential-thinking`.
3. No passo de *Analysis*, o agente percebe que usar `std::sync::Mutex` com `.await` no Tokio causará *deadlocks*. Ele altera a rota no MCP para usar `tokio::sync::Mutex` e `mpsc::channel`.
4. O MCP finaliza a *Conclusion*.
5. O agente responde ao usuário: *"Raciocínio concluído. Identifiquei que precisamos de canais MPSC isolados com `tokio::sync::Mutex` para evitar deadlocks no Event Loop. Deseja que eu inicie a implementação desta arquitetura?"*