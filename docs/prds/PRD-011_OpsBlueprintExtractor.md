# PRD-011: OpsBlueprintExtractor (N11)

## 1. Visão Geral
**Status:** Fase B (Especificação)
**Nó do DAG:** N11
**Objetivo:** Identificar e extrair artefatos de infraestrutura, orquestração e CI/CD (ex: Dockerfiles, Makefiles, GitHub Actions workflows) do repositório. O foco é alimentar o SODA SSOT com os projetos de infraestrutura (Blueprints) sem incorrer em penalidades de I/O desnecessárias ou estouro de RAM.

## 2. Contrato de I/O (Interface)

### 2.1. Entradas (Input)
A execução opera diretamente no sistema de arquivos local (Ramdisk/SSD) sem acionar o Sandbox (Sidecar), reaproveitando a interface imutável do nó N8 (ManifestExtractor).
- `repo_path: &Path`: Caminho absoluto imutável para a raiz do repositório a ser extraído.

### 2.2. Saídas (Output)
A função de extração deverá retornar a seguinte assinatura formal:
`Result<OpsPayload, ExtractionError>`

#### 2.2.1. `OpsPayload` (Sucesso)
Struct imutável padronizada:
- `infra_files: Vec<InfraFile>`
  - `InfraFile`: Struct com os campos `path: String` (caminho relativo à raiz) e `content: String` (o código raw do blueprint extraído).

#### 2.2.2. `ExtractionError` (Falha)
Reaproveita o enum já consolidado no `extract.rs` (do nó N8), suportando erros de I/O nativos, problemas de path resolution e interrupções por limites termodinâmicos (FileTooLarge).

## 3. Cenário Principal de Falha (Fail-Fast Termodinâmico)

**O Problema:** Um arquivo `.github/workflows/gigante.yml` corrompido ou um `Dockerfile` malicioso (ex: log concatenado junto de 20MB) pode asfixiar a RAM (limite de 6GB da dGPU/sistema), causando um Out-Of-Memory (OOM) fatal ao tentar alocar buffers na Thread Principal do Tokio.
**O Paradigma:** Fail-Fast Protetivo (Aborto Imediato de Subtarefa).
**A Solução:** Ao identificar um arquivo de infraestrutura alvo, **antes** de qualquer leitura de bytes ou bufferização, o extractor DEVE checar os metadados do filesystem. Se o arquivo ultrapassar o teto estrito estabelecido (ex: 1MB), o processamento deste alvo é ABORTADO sumariamente, disparando um `ExtractionError::FileTooLarge`. O sistema rejeita o artefato tóxico garantindo a integridade termodinâmica da máquina para preservar os demais nós do Harvester.

## 4. Invariantes de Arquitetura e Proibições Tóxicas

A implementação do nó N11 deve observar obediência cega às seguintes diretrizes:

### 4.1. PT-OPS-1 (Zero Recursão Profunda)
É TERMINANTEMENTE PROIBIDO instanciar varreduras de árvore massivas em todo o repositório utilizando crates como `walkdir` ou `jwalk`. A varredura de infraestrutura é cirúrgica e de complexidade $\mathcal{O}(1)$ focada em caminhos estáticos:
- Busca EXATA apenas na raiz: `Dockerfile`, `docker-compose.yml`, `docker-compose.yaml`, `Makefile`.
- Busca rasa com Teto de Profundidade: EXATAMENTE 1 nível dentro da pasta `.github/workflows/`. A lógica de leitura de diretório (`read_dir`) NÃO PODE descer subdiretórios encontrados ali dentro.

### 4.2. PT-OPS-2 (Trava de RAM Inegociável)
A leitura de conteúdos é blindada matematicamente. É **OBRIGATÓRIA** a invocação de `tokio::fs::metadata(path).await?.len()` para aferir o peso atômico do artefato ANTES da chamada para `tokio::fs::read_to_string`. Nenhuma alocação ocorre sem a validação do peso atômico do alvo.

### 4.3. PT-3 (Zero Bloqueio no Event Loop)
Toda interação direta com o sistema de arquivos deve ser estritamente não-bloqueante via syscalls async. A varredura rasa (`tokio::fs::read_dir`), a extração de metadados (`tokio::fs::metadata`) e a ingestão (`tokio::fs::read_to_string`) NUNCA podem invocar a camada `std::fs` bloqueante, protegendo as threads do executor.

## 5. Critérios de Conclusão (Definition of Done - DoD)
- [ ] Módulo compartilha/amplia harmoniosamente o ecossistema `extract.rs` do N8.
- [ ] Extrator localiza perfeitamente alvos estáticos predeterminados na raiz (`Dockerfile`, `docker-compose.yml`, `Makefile`).
- [ ] Rotina explora estritamente a profundidade rasa (depth 1) da pasta `.github/workflows/`.
- [ ] Invariante `tokio::fs::metadata(path).await?.len()` < 1MB implementado de fato antes da alocação.
- [ ] Testes da Fase C simulam arquivos > 1MB via I/O virtual ou temporário, demonstrando que eles acionam falha `FileTooLarge` rápida.
- [ ] Zero dependências importadas de pacotes `std::fs` bloqueantes para varredura ou leitura.
