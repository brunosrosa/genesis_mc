# ADR-008-Roteamento-FinOps

## Status
Aceito (Ativo e Inegociável)

## Contexto
O ecossistema SODA opera em um regime no qual o uso impensado de modelos em nuvem premium (ex: Claude 3.5 Sonnet, GPT-4o) para processamento de contextos imensos (como refatorações de repositórios inteiros contendo dezenas de milhares de tokens) provoca custos financeiros proibitivos por chamadas de API de nuvem convencionais (Inference Bill Shock). Paralelamente, forçar tarefas complexas a rodar inteiramente na dGPU local de 6GB da RTX 2060m causa travamentos por Out-of-Memory (OOM) e degradação severa da latência e da vida útil do hardware.

## Decisão
Implementar a arquitetura de **Roteamento Híbrido FinOps** governada pelo algoritmo **ParetoBandit** no AgentGateway local compilado em Rust:
1. **Métrica E³ (Efficiency-aware Effectiveness Evaluation):** A tomada de decisão de roteamento não usa regras estáticas de intenção. O ParetoBandit calcula em tempo real o balanceamento ótimo de **Custo vs. Qualidade vs. Latência** para cada tarefa específica.
2. **MoE Multi-Nível Hierárquica:**
   - **Nível 0 (Triagem Local na CPU):** Executado em $< 50ms$ na CPU i9 por um classificador local estruturado. Tarefas mecânicas simples ou buscas locais triviais resolvem-se localmente a custo US$ 0,00.
   - **Nível 1 (Edge Node na dGPU):** Contextos normais ($< 8000$ tokens) e refatorações atômicas seguras rodam localmente na RTX 2060m aproveitando modelos especialistas locais quantizados, mantendo soberania de dados e custo US$ 0,00.
   - **Nível 2 (Subscription Hacking / Cloud Fallback):** Tarefas complexas de raciocínio de longo horizonte sofrem fallback tático para APIs de nuvem asiáticas baratas de lote (Batch APIs) ou são despachadas para sidecars efêmeros conectando as CLIs oficiais instaladas na máquina do usuário (ex: Gemini CLI / Claude Code CLI). Isso redireciona a carga de processamento massivo para as cotas mensais de assinatura fixa (Flat-Rate) do usuário, anulando faturamentos marginais variáveis.
3. **Disjuntores FinOps (Circuit Breakers):** Um firewall financeiro monitora o consumo cumulativo diário de tokens e custos. Se o teto configurado for ameaçado, o disjuntor desarma, cessando qualquer requisição externa e travando as rotinas no motor local.

## Consequências
- **Faturamento Marginal Zero:** Refatorações de centenas de milhares de tokens são executadas sem sobressaltos e sem gerar faturas variáveis de API para o usuário.
- **Termodinâmica Preservada:** A GPU RTX 2060m local opera sob stress controlado, evitando aquecimento extremo e estrangulamento por uso contínuo de RAM.
- **Autonomia Inteligente:** A IA decide autonomamente qual é a melhor máquina de raciocínio para a tarefa sem incomodar ou demandar intervenções manuais cognitivas de roteamento pelo usuário.

## Restrições Bare-Metal
- **Latência de Decisão do ParetoBandit:** A avaliação da equação e o despacho do roteamento devem executar em menos de **10ms** no Gateway Rust.
- **Disjuntor de Teto Diário:** Interrupção compulsória de chamadas externas de nuvem se o gasto diário de tokens atingir o teto parametrizado pelo usuário.
- **Igiene de Conexão:** Quedas de rede disparam fallback automático imediato para inferência local com emissão de status estático na Bottom Bar.
- **Teto de Paralelismo na dGPU:** É proibido carregar múltiplos agentes em paralelo na RTX 2060m; impor **Batching Sequencial** com reciclagem de contexto via **FastSwitch/KVCOMM**.
- **Git I/O fora do Event Loop:** Operações de commit/snapshot via **gitoxide** (I/O intensivo) são proibidas no Event Loop principal; devem rodar estritamente fora do loop assíncrono (thread dedicada/worker), comunicando-se por filas/canais.
