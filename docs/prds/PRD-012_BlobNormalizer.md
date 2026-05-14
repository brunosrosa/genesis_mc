# PRD-012: BlobNormalizer (N12)

## 1. Visão Geral
**Status:** Fase B (Especificação)
**Nó do DAG:** N12
**Objetivo:** Atuar como o injetor atômico de dados (sink) do Harvester. O BlobNormalizer recebe os artefatos puros extraídos e fatiados na RAM (manifestos, metadados sociais, blueprints) e os persiste de forma cirúrgica e fragmentada no armazenamento episódico (SQLite), preparando o terreno para leitura isolada pelas Lentes Cognitivas sem engasgar o pipeline com payloads mortos.

## 2. Contrato de I/O (Interface)

### 2.1. Entradas (Input)
A rotina receberá como injeção de estado e controle:
- `repo_id: &str`: Identificador canônico do repositório sendo processado.
- `blobs: Vec<ArtifactBlob>`: O array dinâmico com os fatiamentos atômicos contendo a estrutura:
  ```rust
  pub struct ArtifactBlob {
      pub artifact_type: String, // ex: "AST", "Manifest", "OpsBlueprint", "CommunityMeta"
      pub payload_blob: Vec<u8>, // A alma matemática binária ou serializada
  }
  ```
- `db_pool: &SqlitePool`: Referência ao executor de banco de dados (ex: `sqlx::SqlitePool`) gerenciado no estado global, aderindo ao padrão Zero-Garbage.

### 2.2. Saídas (Output)
A função de inserção deverá ser estritamente void na conclusão:
`Result<(), HarvesterError>`
O `HarvesterError` deverá possuir variantes que cubram falhas transacionais de banco de dados.

## 3. Cenário Principal de Falha (Aborto Transacional por Contenção)

**O Problema:** Durante a inserção do `Vec<ArtifactBlob>`, o SQLite pode enfrentar um timeout por lock (`SQLITE_BUSY` prolongado), ou o disco NVMe pode encher repentinamente.
**O Paradigma:** Atomicidade Rígida (All-or-Nothing).
**A Solução:** A inserção do lote completo de *blobs* de um repositório DEVE ser empacotada OBRIGATORIAMENTE em uma única transação atômica explícita. Se a persistência de qualquer índice do vetor falhar por disco corrompido ou lock inquebrável, a operação dispara um `HarvesterError::StorageError` executando o `ROLLBACK` atômico. Repositórios fragmentados são rejeitados da base, preservando o banco estéril.

## 4. Invariantes de Arquitetura e Proibições Tóxicas

A implementação do nó N12 deve curvar-se sem restrições às seguintes regras:

### 4.1. PT-BLOB-1 (Morte Absoluta ao JSON Gigante)
É TERMINANTEMENTE PROIBIDO criar um mega-objeto JSON unificando tudo e descarregá-lo no banco. O BlobNormalizer tem a OBRIGAÇÃO de inserir CADA `ArtifactBlob` como uma TUPLA INDIVIDUAL na tabela `artefatos_brutos`.
- **Schema OBRIGATÓRIO da Tabela:** `repo_id`, `artifact_type`, `payload_blob`.
Isso habilita o Roteamento Semântico futuro a dar `SELECT payload_blob WHERE artifact_type = 'OpsBlueprint'` e extrair as fatias da árvore sob complexidade $\mathcal{O}(1)$, deixando o lixo de outros artefatos no disco para não saturar a CPU.

### 4.2. PT-3 (Zero Bloqueio do Tokio)
Inserções SQLite são operações síncronas no núcleo do SO (`fsync`). É proibido travar o Event Loop do Tokio com chamadas não escalonáveis. O código DEVE usar conectores I/O puramente assíncronos (como `sqlx` assíncrono sobre sqlite) ou empurrar explicitamente a transação atômica bruta para os workers via `tokio::task::spawn_blocking` se o drive for síncrono (ex: `rusqlite`).

### 4.3. PT-2 (Zero Arquivos no Disco Host)
O pipeline é imaculado. A rotação dos dados nasce na RAM (via extração ou `snapsafe`) e vai diretamento pelo cabo TCP/socket nativo ao núcleo da DB. ESTÁ ESTRITAMENTE BANIDO o uso de buffers físicos como o despejo num arquivo `_RAW_DATA.json` temporário apenas para contornar problemas de alocação de struct. RAM -> Engine SQLite.

## 5. Critérios de Conclusão (Definition of Done - DoD)
- [ ] A struct `ArtifactBlob` encapsula vetores/arrays estritos na definição.
- [ ] Testes provam que múltiplos blobs num mesmo `repo_id` resultam em inserção multilinha, validando a morte do JSON gigante.
- [ ] Transação atômica cobre todo o for/loop da inserção; nenhum commit parcial é admitido.
- [ ] Falhas simuladas no SQL estouram erro de `HarvesterError::StorageError`.
- [ ] Zero uso de API `std::fs` para escrever buffers raw na máquina do host.
