# PRD-018: SsotInjector (N18 - Injetor SSOT)

## 1. Visão Geral
O `SsotInjector` marca o encerramento do *pipeline* cognitivo. Operando como a ponte entre o *Bare-Metal* e o *Single Source of Truth* (SSOT) em nuvem, ele orquestra a Execução Durável local e despacha a carga atômica de sabedoria condensada para as matrizes operacionais do SODA (Google Sheets). 

## 2. Assinatura do Contrato

### Entrada
- `repo_id: String`: O identificador chave do repositório no banco de dados local.
- `payload: SgrPayload`: A struct rigidamente tipada e purificada pelas leis do *Schema-Guided Reasoning* (Fase 3).

### Saída
- `Result<(), SsotError>`: O retorno indica se a dupla transação (SQLite + Despacho para a Nuvem) foi validada. Falhas retornam instantaneamente a propagação de erro, interrompendo falsos sucessos.

## 3. Fluxo de Carga Atômica

### Selagem da Execução Durável (Memória L2)
Antes de enfrentar a instabilidade de rede da nuvem, o sistema DEVE fechar a transação no ambiente controlado.
- **Ação Obrigatória:** O injetor deve executar um `UPDATE` no `soda_heuristic_vault.db` da tabela `repositorios`, cravando o `status_processamento` como `CONCLUIDO`. Se a gravação no disco local falhar, o fluxo é imediatamente abortado, preservando a idempotência.

### Roteamento Multi-Aba (O Prisma)
Ao espelhar os dados para a nuvem, o `SgrPayload` monolítico é fragmentado e roteado logicamente para 4 vertentes especializadas:
1. **MASTER_SOLUTIONS_v3:** Matriz primária de telemetria, notas (`score_final`) e o `executive_verdict`.
2. **SODA_GRAPH_TOPOLOGY:** Isolamento descritivo da pilha tecnológica e topologia para análise do Chyros Daemon.
3. **ACTION_MATRIX:** Vertente tática. Armazena as decisões cruciais caso a `cannibalization_action` envolva `AbsorverLogica` ou `ExtrairScripts`.
4. **QUARANTINE_RADAR:** Doca de quarentena. Dados são injetados aqui caso haja detecção de "design misuse risk" ou violações arquiteturais tóxicas, sinalizando alerta vermelho para o Arquiteto.

## 4. Invariantes de Blindagem (Proibições Tóxicas)
- **PT-SSOT-1 (Fobia de Chamadas Consecutivas):** As APIs de planilhas em nuvem (Google Sheets API) possuem Rate Limits agressivos de 60 RPM, disparando o fatídico Erro HTTP 503. Por conta disso, é **TERMINANTEMENTE PROIBIDO** utilizar loops ou chamadas seriais (ex: `add_row` sequenciais para cada aba). A regra de sobrevivência na Fase 4 é a agregação total de memória. O nó construirá o dicionário massivo das quatro fatias internamente no Rust e emitirá **UMA ÚNICA CHAMADA ATÔMICA** via HTTP (mimetizando a carga explosiva de um `batch_update_cells`).
