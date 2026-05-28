# ADR-020-Enxame-Cognitivo-ELO

## Status
Aceito (Ativo e Inegociável)

## Contexto
O viés de automação (*Automation Bias*) induz o usuário a confiar cegamente e passivamente em decisões estocásticas unilaterais de IAs. Em tarefas arquiteturais densas, agentes convencionais que aceitam premissas ingênuas sem questionar geram códigos errôneos ("slop"), provocando bugs ocultos severos que degradam o compilador. Além disso, a fadiga de aprovação humana ao ter que revisar dezenas de pequenas correções repetitivas de IA reduz a produtividade e a saúde neurocognitiva do usuário.

## Decisão
Implementar a arquitetura de **Enxame Cognitivo Baseado em Reputação ELO** no SODA:
1. **Debate Multi-Agente Invisível (Free-MAD):** Antes de propor qualquer plano de mutação física de arquivos, o core instancia um debate interno invisível na CPU. Duas personas intelectuais opostas (ex: IA Otimista Construtora vs. IA Auditora Bare-Metal Implacável) debatem as premissas e tentam provar ativamente como a proposta falhará nos limites físicos locais.
2. **Map-Reduce Socrático:** O fluxo causal de raciocínio decompõe a tarefa complexa em ramos isolados de análise (Map), submete cada bloco ao escrutínio cruzado de falsificação física (Cross-Critique) e consolida a conclusão refinada de forma sintética (Reduce) na CPU.
3. **Torneios ELO baseados em Co-Scientist:** A competência e o histórico de sucesso dos agentes e templates de prompts são rastreados de forma contínua por algoritmos estatísticos. A taxa de aprovação de código gera uma Média Móvel Exponencial (EMA/ELO) de confiança:
   - ** HOTL (On-The-Loop) Autônomo:** Se a EMA de ELO do agente para aquela classe de tarefas for $> 0.94$, ele adquire autonomia transitória para operar em background.
   - ** HITL (In-The-Loop) Compulsório:** Se o linter quebrar, ocorrer falha de TDD ou um desvio de comportamento anômalo (Z-Score severo) for detectado pelo algoritmo de Welford, o score ELO zera instantaneamente. A ação é congelada no Canvas e requer aprovação explícita na Agent Inbox.
4. **Modelos de Linguagem Recursivos (RLM):** Para tarefas massivas sem estourar VRAM, o Enxame adota delegação Map-Reduce via RLMs. Um LLM atua num loop **REPL** quebrando tarefas colossais e delegando fatias para **Sub-RLMs efêmeros** rodando em **Wasmtime**, agregando apenas os resultados.

## Consequências
- **Erradicação do Falso Consenso:** Decisões de IA são robustas, auditadas e altamente blindadas contra erros simplórios de raciocínio estocástico.
- **Equilíbrio Cognitivo do Usuário:** O Arquiteto Humano é poupado de microssegundos de aprovações repetitivas de tarefas de baixo risco, focando a atenção estritamente em anomalias críticas de sistema.
- **Segurança de Código:** Menores taxas de regressão sistêmica e quebras do compilador Rust.

## Restrições Bare-Metal
- **Teto do Free-MAD:** O debate interno invisível é limitado ao teto estrito de **5 iterações cognitivas** assíncronas para poupar a CPU i9 e evitar loops infinitos.
- **Rastreabilidade ELO:** Os scores estatísticos de ELO e EMA são salvos como eventos imutáveis no SQLite transacional (L2) e auditados incrementalmente.
- **Bloqueio de Autonomia:** Nenhuma autonomia HOTL é concedida a tarefas com BLAST_RADIUS alto (ex: mutação direta em schemas do DB de produção ou modificação de middlewares de sandboxing).
- **Teto de Paralelismo na dGPU:** É proibido carregar múltiplos agentes em paralelo na RTX 2060m; impor **Batching Sequencial** com reciclagem de contexto via **FastSwitch/KVCOMM**.
- **Git I/O fora do Event Loop:** Operações de commit/snapshot via **gitoxide** (I/O intensivo) são proibidas no Event Loop principal; devem rodar estritamente fora do loop assíncrono (thread dedicada/worker), comunicando-se por filas/canais.
- **Sub-RLMs em Wasmtime:** Sub-RLMs efêmeros devem rodar estritamente em **Wasmtime** e comunicar-se com o core apenas por resultados compactos (sem refluir contextos colossais), preservando VRAM e evitando pressão de I/O.
