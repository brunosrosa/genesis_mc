# ADR-009-Decodificacao-Restrita

## Status
Aceito (Ativo e Inegociável)

## Contexto
Durante tarefas críticas de extração estruturada de dados (ETL Cognitivo) e geração de Grafos Acíclicos Dirigidos (DAGs) de fases, a inferência estocástica livre frequentemente gera erros de sintaxe (Markdown poluído, JSON inválido ou chaves ausentes). Esses pequenos desvios de formatação quebram os analisadores lógicos nativos em Rust, forçando re-tentativas de inferência desgastantes que consomem tokens desnecessários e geram atrito operacional na esteira de montagem de código.

## Decisão
Fica rigidamente decretada a adoção de **Decodificação Restrita (Constrained Decoding)** em todo o core do SODA:
1. **O Motor llguidance:** A inferência local e o recebimento de respostas do gateway de inferência devem operar rigidamente guiados pela biblioteca **llguidance** implementada em Rust.
2. **Autômatos de Gramática Livre de Contexto (CFG):** O processamento da geração de tokens na dGPU/CPU é interceptado em tempo real de execução. O motor llguidance restringe dinamicamente a amostragem de logits, forçando a saída do modelo a se conformar perfeitamente a uma Gramática Livre de Contexto compilada a partir de schemas JSON estritos (esquemas Pydantic exportados ou esquemas locais JSON-Schema).
3. **Erradicação de Cascas Textuais:** Fica terminantemente proibido o uso de prompts livres de instrução estruturada demandando JSON em texto aberto. O modelo local é matematicamente incapaz de emitir caracteres supérfluos, narrativas fora do schema ou barras de escape inválidas.

## Consequências
- **Precisão Mecânica de 100%:** O payload retornado do motor de inferência é garantido sintaticamente íntegro em nível de byte e bit em relação ao esquema solicitado, eliminando totalmente falhas de parsing.
- **Eficiência de Geração:** A IA não gasta VRAM ou tempo gerando textos introdutórios ou tags de fechamento supérfluas (ex: "Aqui está o seu JSON: ..."), acelerando o tempo de processamento em até 30%.
- **Segurança de Execução:** Menores riscos de ataques de injeção de prompt estruturada que tentem corromper ou sequestrar as vias de comando da aplicação.

## Restrições Bare-Metal
- **Latência de Interceptação llguidance:** O tempo de processamento e validação do autômato gramatical de tokens pelo llguidance deve executar em menos de **50 microssegundos (50µs)** por token gerado.
- **Consistência de Schema:** Todas as DTOs e esquemas de dados da aplicação devem obrigatoriamente exportar seus arquivos JSON-Schema para a pasta de especificações (`docs/specs/`) para governar os validadores locais.
- **Fail-Closed Gramatical:** Se o autômato falhar ao harmonizar a geração local com o schema por estouro de cota de tokens, a operação é abortada sem persistência corrompida.
