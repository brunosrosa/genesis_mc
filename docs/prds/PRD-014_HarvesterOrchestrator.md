# PRD-014: HarvesterOrchestrator (N14)

## 1. Visão Geral
**Status:** Fase B (Especificação)
**Nó do DAG:** N14 (Maestro da Fase 1)
**Objetivo:** Atuar como a "Placa-Mãe" determinística do pipeline de extração de dados (SODA ETL). Este orquestrador coordena o ciclo de vida completo do processamento de um repositório, desde a alocação física de memória (Ramdisk) até a injeção atômica no banco de dados, garantindo a execução paralela de extratores e a higienização infalível (Purga) ao final da execução.

## 2. Contrato de I/O (Interface)

### 2.1. Entradas (Input)
A rotina recebe as coordenadas iniciais e a via de persistência:
- `repo_url: &Url`: A URL (ou URI) do repositório alvo a ser clonado e processado.
- `db_pool: Arc<Mutex<rusqlite::Connection>>`: O executor do banco de dados (SQLite) para a injeção atômica final.

### 2.2. Saídas (Output)
A função retorna o veredito da orquestração:
`Result<(), OrchestratorError>`
O sucesso indica que o repositório foi processado (mesmo que com falhas parciais em alguns extratores). O erro estrutural indica uma falha de "Aborto Preemptivo" (ex: falta de RAM, host inatingível) que impede o início da análise.

## 3. Fluxo Assíncrono Obrigatório (A "Placa-Mãe")

O orquestrador DEVE seguir estritamente o pipeline acíclico:

1. **Setup Físico (Fail-Fast):** Aciona o `RamdiskAllocator` [N1]. Se não houver memória suficiente, aborta preemptivamente.
2. **Ingestão:** Aciona o `BloblessCloner` [N2] montando o clone do repositório no Ramdisk.
3. **Isolamento:** Ergue o ambiente controlado via `SandboxOrchestrator` [N3].
4. **Extração Concorrente:**
   - O `ExtractionRouter` [N5] é despachado de forma assíncrona para coordenar os sub-nós locais: [N6 (AST)], [N7 (Docs)], [N8 (Manifests)], [N9 (StaticAnalysis)] e [N11 (OpsBlueprint)].
   - Simultaneamente, o orquestrador dispara em paralelo (via `tokio::join!`) o `CommunityMetaFetcher` [N10] para buscar as métricas sociais na rede.
5. **Consolidação Atômica:** O orquestrador coleta todos os `ArtifactBlob` bem-sucedidos das extrações e os injeta no `BlobNormalizer` [N12] para gravação no SQLite.
6. **GARANTIA DE VIDA (Lifeline Incondicional):** Após a consolidação, seja o fluxo concluído com sucesso total, falha parcial ou pânico num extrator, o orquestrador invoca compulsoriamente o `PurgeGuard::purge(sandbox, ramdisk)` [N13]. Esta barreira assegura que o SO host nunca sofra vazamentos do pipeline.

## 4. Cenários de Falha e Resiliência

- **Aborto Preemptivo:** Se a alocação de infraestrutura falhar (Ex: `RamdiskAllocator` lança `InsufficientMemory`), o orquestrador aborta a extração antes de começar, retornando o `OrchestratorError` e garantindo que o pool de processamento não perca tempo com um repo inviável.
- **Falha Parcial (Degradação Graciosa):** Se o `StaticAnalysisSidecar` [N9] entrar em colapso devido a uma sintaxe alienígena, a exceção é contida. O orquestrador coleta os blobs que sobreviveram (ex: metadados do [N10] e manifestos do [N8]) e os envia ao `BlobNormalizer` [N12]. O pipeline prossegue e o repositório é indexado parcialmente.

## 5. Invariantes de Arquitetura e Proibições Tóxicas

### 5.1. PT-ORCH-1 (Zero IA)
É **TERMINANTEMENTE PROIBIDA** a invocação de LLMs, geração de texto via Prompts, embeddings ou lógicas probabilísticas dentro deste nó. O `HarvesterOrchestrator` é um motor de força bruta estritamente determinístico, lidando com bytes, File Systems, processos do OS e conexões de rede padrão.

### 5.2. PT-3 (Zero Bloqueio)
O pipeline mestre DEVE rodar livremente no executor do Tokio. Bloqueios inerentes à persistência final do `BlobNormalizer` [N12] devem estar obrigatoriamente envelopados em um `spawn_blocking`. O orquestrador nunca sequestra as threads de I/O assíncrono.

## 6. Critérios de Conclusão (Definition of Done - DoD)
- [ ] Implementação de concorrência real entre as tarefas de disco (ExtractionRouter) e de rede (CommunityMetaFetcher).
- [ ] Falhas em extratores específicos não causam o pânico geral da "Placa-Mãe" (Falha Parcial permitida).
- [ ] O `PurgeGuard` é invocado de forma infalível ao término do escopo da extração.
- [ ] PT-ORCH-1 respeitada (código sem crates ou referências a inteligência artificial generativa).
