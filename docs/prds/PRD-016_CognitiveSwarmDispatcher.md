# PRD-016: CognitiveSwarmDispatcher (N16 - Enxame Cognitivo)

## 1. Visão Geral
O `CognitiveSwarmDispatcher` atua como o regente mestre da Fase 2 do SODA ETL. Sua função não é extrair dados brutos, mas instanciar o paradigma matemático **Free-MAD** (Consensus-Free Multi-Agent Debate). Ele quebra o problema em três linhas paralelas de raciocínio, impedindo o consenso prematuro e o colapso de pensamento único (Echo Chamber), tudo governado pelas restrições termodinâmicas do `IronCostBreaker`.

## 2. Assinatura do Contrato

### Entrada
- `repo_id: String`: Chave estrangeira para recuperar os blobs (`Manifest`, `OpsBlueprint`, `CommunityMeta`) gerados na Fase 1 e armazenados no banco SQLite.
- `soda_canon_raw: &str`: Regras estáticas carregadas em RAM que guiam o crivo socrático do SODA.

### Saída
- `Result<(), SwarmError>`: Sucesso indica que as reflexões textuais das três inteligências foram depositadas atomicamente na tabela SQLite `debates_enxame`. Em caso de estouro termodinâmico ou de falha transacional, o erro aborta o processo de imediato.

## 3. Orquestração e Paralelismo (O Motor Tokio)

### Catraca FinOps
Antes de montar requisições pesadas ou ocupar banda de rede, o despachante calcula a densidade de tokens dos artefatos recuperados.
- Ele **DEVE** acionar a chamada puramente O(1) do `IronCostBreaker::calculate_and_route`.
- Se a rota for negada (`FinOpsError`), o despachante paralisa e morre graciosamente.
- Se aprovada, o `AllowedRoute` ditará para onde a requisição HTTP será direcionada (Nuvem via HTTPS ou Inferência Local HTTP/Wasm).

### Mecânica Free-MAD (Tripartite)
A extração cognitiva deve criar 3 requisições isoladas, que nunca compartilham contexto entre si durante a inferência:
1. **Lente A (Sentido - Produto/UX):** Focada na proposta de valor, fricção adaptativa e experiência do usuário final.
2. **Lente B (Estrutura - Arquitetura/Bare-Metal):** Focada na pilha tecnológica, isolamento IPC, VRAM e blindagem Zero-Trust.
3. **Lente C (Realidade - Operação/FinOps):** Focada no custo de manutenção, gargalos operacionais e viabilidade comercial do repositório.

O agrupamento das requisições **DEVE INEGOCIAVELMENTE** ser engatilhado de forma simultânea via concorrência estruturada do `tokio::join!`, garantindo latência mínima e eficiência máxima de *Network I/O*.

## 4. Invariantes de Blindagem (Proibições Tóxicas)
- **PT-SWARM-1 (Zero Formatação Complexa):** As Lentes da Fase 2 são operários socráticos e filosóficos, não parsers. É **TERMINANTEMENTE PROIBIDO** forçar a nuvem a estruturar as respostas em JSON massivo, JSON Schema, ou Pydantic na saída deste nó. Impor formatação estruturada consome energia de inferência e degrada a capacidade de dedução do modelo. Os prompts devem exigir explicitamente: *"Responda apenas em texto livre formatado com bullets e focado na argumentação"*.
- *Nota Arquitetural:* A extração e preenchimento determinístico das tabelas via SGR (Schema-Guided Reasoning) serão realizados exclusivamente na Fase 3.
