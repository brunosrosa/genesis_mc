# PRD-015: IronCostBreaker (N15 - Disjuntor FinOps)

## 1. Visão Geral
O `IronCostBreaker` atua como o Guardião Financeiro (Disjuntor FinOps) na transição entre a Fase 1 (Harvester) e a Fase 2 (Cognição). Sua missão exclusiva é proteger o sistema contra o "Subscription Hacking" e faturamentos astronômicos de APIs LLM, governando matematicamente o roteamento do `ParetoBandit`.

## 2. Assinatura do Contrato

### Entrada
- `tokens_count: usize`: A estimativa física do volume de tokens a serem consumidos (baseado no payload extraído do N12).
- `target_tier: ModelTier`: A camada de inteligência desejada para a tarefa. Composta pelo Enum:
  - `PremiumCloud` (Modelos de fronteira, alto custo)
  - `FlashCloud` (Modelos otimizados/batch, baixo custo)
  - `LocalGPU` (RTX 2060m, custo zero)

### Saída
- `Result<AllowedRoute, FinOpsError>`: A rota definitiva aprovada. Em caso de estouro de orçamento recuperável, o disjuntor pode forçar um `FallbackToLocal`.

## 3. Lógica Matemática O(1)
Para garantir execução imediata sem atrasos de rede, o disjuntor opera estritamente com matemática estática baseada em microdólares (para evitar perda de precisão em Float).

- **Tabela de Custos Estática:** 
  - A conversão de tokens em custo baseia-se em constantes hardcoded (Custo por Milhão de Tokens). Exemplo:
    - PremiumCloud = 15 USD / 1M.
    - FlashCloud = 0.5 USD / 1M.
    - LocalGPU = 0.0 USD.
- **Cálculo:** `cost_micro_usd = (tokens_count * cost_per_1m_micro_usd) / 1_000_000`.
- **Aprovação:** A requisição é liberada (`AllowedRoute`) caso `cost_micro_usd` se mantenha abaixo do teto contábil global estrito: `MAX_DAILY_BUDGET_MICRO_USD`.
- Caso `target_tier == LocalGPU`, a aprovação possui curto-circuito (Short-Circuit) e é garantida imediatamente por ter custo zero.

## 4. Cenário de Falha e Colapso
O cenário crítico de falha ocorre quando a Matemática e o Hardware entram em conflito sem solução:
- O faturamento projetado na nuvem ultrapassa o `MAX_DAILY_BUDGET_MICRO_USD`.
- Porém, o `tokens_count` supera drasticamente os limites do KV Cache da VRAM (ex: `> 16000 tokens`), impossibilitando o `FallbackToLocal` sem causar Out-Of-Memory (OOM) no *Bare-Metal*.
- **Comportamento Exigido:** O disjuntor trava a porta da Fase 2 e cospe o erro estrutural `FinOpsError::BudgetExceeded`. Nenhuma conta da nuvem é usada, e a placa de vídeo local é protegida. O pipeline para e demanda auditoria humana.

## 5. Invariantes de Blindagem (Proibições Tóxicas)
- **PT-FIN-1 (Zero API Call):** É TERMINANTEMENTE PROIBIDO que o disjuntor execute requisições HTTP (API) para checar o preço atual de tokens do provedor. A tabela de custos DEVE estar matematicamente fixada no binário. A decisão é estritamente síncrona, CPU-bound e tem complexidade temporal de latência indetectável O(1).
