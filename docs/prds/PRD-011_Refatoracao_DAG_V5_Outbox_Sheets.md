# PRD-011: Refatoração do DAG V5 com Outbox Pattern (SQLite -> Google Sheets)

## 1. Objetivo e Contexto
O fluxo de ignição de repositórios do SODA (Fases N0, N1 e N2) atualmente sofre com gargalos de I/O síncrono e concorrência direta na API do Google Sheets. A escrita e leitura célula a célula geram alta latência e expõem a infraestrutura a erros frequentes de Rate Limit (HTTP 429). 

Como o Google Sheets é nossa principal "Janela de Vidro" (Dashboard visual ao vivo) na ausência de um frontend final, precisamos preservar seu papel de espelho de estados (PENDENTE $\rightarrow$ INICIAR_TRIAGEM $\rightarrow$ TRIAGEM_CONCLUIDA).

A solução é implementar o **Outbox Pattern**. As fases N1 (Guardião) e N2 (Batedor) passam a operar como motores de execução local-first, lendo e escrevendo estados primordialmente em tempo $\mathcal{O}(1)$ no SQLite (`soda_heuristic_vault.db`). Um agente/thread injetor assíncrono monitora as mutações de estado locais e as propaga em lotes (batch) ou sequencialmente com Jitter/Sleep obrigatório de 1000ms a 2000ms para o Google Sheets.

---

## 2. Topologia do Outbox Pattern (SQLite -> Google Sheets)

O diagrama abaixo ilustra o desacoplamento de escrita e leitura síncrona. Toda a inteligência e máquina de estados roda localmente no SQLite. A sincronização com a nuvem ocorre de forma assíncrona por trás de uma fila de Outbox cadenciada.

```mermaid
flowchart TD
    subgraph Local_First_Engine [Local-First Engine (SQLite)]
        DB[(soda_heuristic_vault.db\ntabela: repositorios)]
        
        N0[N0: Daemon Watcher] -->|1. Descobre link e insere PENDENTE| DB
        N1[N1: Guardião / Fase -1] -->|2. Lê PENDENTE / Atualiza Versão & Status| DB
        N2[N2: Batedor / Fase -0.5] -->|3. Lê INICIAR_TRIAGEM / Roda LLM & Salva Resumo| DB
    end

    subgraph Outbox_Synchronizer [Outbox Synchronizer & Jitter Gate]
        DB -->|4. Detecta linhas dessincronizadas| Sync[Outbox Sync Loop]
        Sync -->|5. Aplica Sleep/Jitter 1000ms - 2000ms| RateLimiter{Rate Limiter}
    end

    subgraph Cloud_Dashboard [Cloud Dashboard]
        RateLimiter -->|6. batchUpdate / Single line write| Sheets[Google Sheets\nMASTER_SOLUTIONS]
    end

    Sheets -.->|7. Novas entradas manuais| N0
```

---

## 3. Preservação Absoluta da Lógica FinOps do Batedor (N2)
A lógica analítica do `f_minus_0_5_batedor_cli.rs` deve ser mantida intacta e protegida contra regressões:
- **Limitação de Entrada:** Truncamento estrito dos READMEs de entrada nos primeiros 3.000 caracteres (`README_CHAR_LIMIT = 3000`).
- **Structured Outputs / Modo Strict:** Chamada via OpenRouter utilizando o modelo `google/gemini-2.5-flash` (ou configurável via env `OPENROUTER_MODEL`), com `temperature = 0.0` e JSON Schema estrito correspondente à estrutura `soda_batedor_triage_v1`.
- **Validação de Categorias:** Classificação estrita pertencente de forma exata a uma das 47 categorias arquiteturais pré-aprovadas.
- **Deduplicação de Sub-Links:** Extração de URLs do README e envio em lote para deduplicação assistida por IA, elegendo e indexando até 12 links-chave de repositórios de referência.

---

## 4. Eliminação de Requisições Síncronas Célula a Célula
- **Fim do `read_sheet_cell`:** Fica terminantemente proibido realizar requisições HTTP individuais de leitura ou escrita célula a célula antes ou após o processamento.
- **Leitura em Batch:** O processo de sincronização com o Sheets (Outbox Sync) deve operar lendo blocos de linhas de uma só vez (ex: `A2:Z100`), comparando o estado local com o estado remoto e gerando um plano de mutação.
- **Escrita Agrupada (batchUpdate):** As escritas no Sheets devem ser consolidadas e enviadas via `batchUpdate` agrupando múltiplas células alteradas em uma única transação de rede por ciclo de sincronização, respeitando a janela de Jitter/Delay.

---

## 5. Definition of Done (DoD) e Metodologia TDD

Para cada um dos executáveis alterados, as seguintes tarefas e testes automáticos devem ser implementados antes de considerar a funcionalidade concluída:

### A. Para o Outbox Sync / Injector (SQLite -> Sheets)
- [ ] Criar testes unitários e de integração mockando a API do Sheets para comprovar que o injetor lê o SQLite local e enfileira as escritas com atraso dinâmico de 1000ms a 2000ms (Jitter).
- [ ] Testar recuperação de erros de rede (ex: HTTP 503 / 429) no injetor aplicando Exponential Backoff sem perder os estados locais pendentes no SQLite.

### B. Para o Guardião (N1) - `f_minus_1_guardian.rs`
- [ ] Modificar o fluxo principal para que ele leia repositórios locais em estado `PENDENTE` no SQLite.
- [ ] Executar o dry-run da tag remota do GitHub.
- [ ] Atualizar localmente no SQLite: `status_processamento` para `'INICIAR_TRIAGEM'` (ou `'DESATUALIZADA'` em caso de drift detectado) e as respectivas versões resolvidas.
- [ ] Validar via testes que nenhum I/O síncrono com o Sheets ocorre durante esta fase.

### C. Para o Batedor (N2) - `f_minus_0_5_batedor_cli.rs`
- [ ] Modificar para processar apenas itens locais com status `'INICIAR_TRIAGEM'` (ou correspondente) no SQLite.
- [ ] Executar a lógica de triagem com IA de forma local e concorrente limitada por semáforo.
- [ ] Gravar a categoria arquitetural e proposta de resumo diretamente no banco de dados local.
- [ ] Atualizar o status local para `'TRIAGEM_CONCLUIDA'`.
- [ ] Assegurar via testes unitários de integração local que a API OpenRouter é chamada com os payloads e limites corretos de caracteres.
