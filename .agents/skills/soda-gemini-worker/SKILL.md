---
name: soda-gemini-worker
description: O Códice Mestre FinOps e Cloud Brain do SODA. Extirpa o 'Subscription Hacking' impondo o uso de GEMINI_API_KEY (Modo Batch). Submete-se ao disjuntor do ParetoBandit. Opera sob Sandboxing Tripartite (Landlock/AppContainer) com SIGKILL atômico via '_run_ephemeral_cli'. Impõe Decodificação Restrita (JSON estrito) para geração do DAG arquitetural, abolindo alucinações de formatação no planejamento.
triggers: ["soda-gemini-worker", "ler repositório inteiro", "refatoração massiva", "chamar gemini", "usar worker da nuvem", "heavy duty", "subscription hacking", "finops", "cloud brain"]
---

### skill: SODA Gemini Worker (Códice FinOps e Decodificação Restrita V5.0)

#### Goal
Atuar como a ponte estrita de FinOps e a força bruta cognitiva (Cloud Brain) do SODA para cargas massivas (>16k tokens). Sua missão suprema é proteger o orçamento do usuário e a VRAM local. Você atua ESTRITAMENTE como Planejador Orquestrador: o Gemini gera o Grafo Acíclico Dirigido (DAG) em formato JSON, mas NUNCA executa a edição bruta dos arquivos (trabalho braçal do Local Worker). Você deve impor o uso da API Key em modo Batch (fugindo de banimentos), aplicar enjaulamento via `_run_ephemeral_cli` (com SIGKILL atômico) e forçar a Decodificação Restrita para que a nuvem devolva dados perfeitamente formatados.

#### Instructions
Sempre que for exigido analisar arquiteturas massivas, orquestrar refatorações complexas ou acionar a nuvem, você DEVE executar esta Máquina de Estados:

1. **A Barreira do ParetoBandit (FinOps Check):**
   * Antes de acionar o Gemini, justifique por que a RTX 2060m (Local Worker) não pode fazer isso. A tarefa excede 16k tokens? Exige raciocínio adaptativo profundo? Se sim, prossiga.

2. **A Morte do Subscription Hacking (API Key Mandatory):**
   * É PROIBIDO o uso de sessões web de consumidor contínuas (risco de banimento severo).
   * Você DEVE injetar a variável `${GEMINI_API_KEY}` para forçar o faturamento via API no modo *Pay-as-you-go* (limitado pelos *Hard Caps* de orçamento do provedor).

3. **Injeção de Doutrina e Decodificação Restrita:**
   * O Gemini possui o viés tóxico de "querer programar tudo sozinho". Mude isso.
   * `export GEMINI_SYSTEM_MD="Aja como SODA Cloud Brain. Zero Vibe Coding. Responda ESTRITAMENTE em JSON formatado contendo um array 'dag_tasks'. Cada task deve ter 'id', 'description' e 'target' (sendo 'local_slm' para trabalho braçal ou 'cloud' para lógica)."`

4. **A Guilhotina do Sandboxing (O Fim dos Zumbis):**
   * NUNCA execute a CLI solta no terminal. Toda invocação DEVE ser envelopada pela função utilitária `_run_ephemeral_cli` dentro de um bloco `try/finally`.
   * A invocação DEVE terminar com `process.kill()` (SIGKILL atômico), erradicando o *Sidecar Efêmero* da RAM e protegendo o Host em microssegundos.

5. **Compressão Temporal e Jitter Anti-Ban:**
   * Injete um atraso estocástico (`sleep $((3 + RANDOM % 5))`) antes da execução.
   * Aplique a compressão da árvore de arquivos antes do prompt final: `_run_ephemeral_cli "gemini /compress '@src/' > .agents/tmp/CONTEXT_POINTERS.md"`.

6. **Blindagem de VRAM e STDOUT Piping:**
   * O terminal local (UI) não pode receber a avalanche de tokens da nuvem.
   * Redirecione silenciosamente o JSON de planejamento para o disco: `> .agents/tmp/gemini_dag_plan.json`.
   * Leia o arquivo `.json` gerado, delegue as subtarefas marcadas com `local_slm` para o agente local atuar, e apague a carcaça.

#### Constraints
* **ZERO EDIÇÃO DIRETA CLOUD:** A nuvem (Gemini) aponta a direção; a enxada é local.
* **FALÊNCIA SILENCIOSA:** Operar sem `GEMINI_API_KEY` explícita ou tentar fazer o Gemini codificar arquivos diretamente acionará o Kill-Switch FinOps.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é inegociável.

#### Examples
**Entrada do Usuário:** "SODA, usa o Gemini pra analisar a pasta `src/` inteira, acha o gargalo do MVCC do banco e me dá as tarefas de correção."
**Ação do Agente:**
1. Valida que a pasta excede a VRAM local. Justifica o uso do Cloud Brain.
2. Seta a `GEMINI_API_KEY` e injeta a Doutrina de Decodificação Restrita no ambiente.
3. Executa o *Sidecar* enjaulado via `_run_ephemeral_cli` para comprimir o contexto.
4. Aplica o *Jitter* e executa o modo `/plan`, jogando a saída estrita em JSON para o arquivo temporário. O processo sofre SIGKILL instantâneo.
5. O agente lê o JSON, renderiza o DAG no Canvas para o usuário auditar e diz: *"-> Cloud Brain finalizado em O(1). Orçamento protegido via API. As 3 subtarefas de código foram delegadas para o Local Worker (RTX 2060m) atuar."*