# PRD-013: PurgeGuard (N13)

## 1. Visão Geral
**Status:** Fase B (Especificação)
**Nó do DAG:** N13 (Finalizador da Fase 1)
**Objetivo:** Atuar como o executor implacável da higiene termodinâmica do SODA ETL. O PurgeGuard atesta o encerramento determinístico do ciclo de vida da extração de um repositório, garantindo a aniquilação atômica do Ramdisk e o sacrifício de Sandboxes de I/O, erradicando qualquer possibilidade de vazamento de RAM ou processos zumbis na máquina do host.

## 2. Contrato de I/O (Interface)

### 2.1. Entradas (Input)
A rotina NÃO aceita referências. Ela absorve a titularidade (Ownership) dos recursos pesados, forçando a transferência de posse por VALOR:
- `sandbox: SandboxHandle`: Manipulador do ambiente de execução e de limites de SO, consumido por valor.
- `ramdisk: RamdiskHandle`: Manipulador do disco temporário em memória RAM, consumido por valor.

### 2.2. Saídas (Output)
A assinatura da função de purga é matematicamente infalível:
`()`
O PurgeGuard jamais retorna um `Result`. Toda e qualquer falha na destruição das instâncias pelo SO deve ser internalizada, convertida em Ghost Telemetry, mas NUNCA propagada como pânico ou erro encadeado. O orquestrador não deve parar porque um lixo recusou-se a morrer instantaneamente.

## 3. Cenário Principal de Falha (Lock Residual de SO Host)

**O Problema:** Durante a tentativa de desmontar o Ramdisk ou purgar os arquivos, o Sistema Operacional do host (ex: Antivírus, Windows Defender ou Indexador de Pesquisa) detém um lock residual exclusivo nos arquivos recém-extraídos, impedindo a exclusão imediata ou o unmount.
**O Paradigma:** Infallible Fail-Soft (Degradação Graciosa no Descarte).
**A Solução:** Ao acionar os métodos de limpeza ou deixar o recurso cair em `Drop`, se ocorrer falha (ex: `Access Denied`), o PurgeGuard proíbe terminantemente o uso de `.unwrap()` ou a interrupção da thread do Tokio. O erro é interceptado e registrado no log passivo (`warn!("PurgeGuard: Fallback no descarte de recursos - lock retido pelo host: {}", e)`). O ciclo do Rust descarta a memória residente (Heap) independente do bloqueio físico no FileSystem virtual.

## 4. Invariantes de Arquitetura e Proibições Tóxicas

O nó N13 opera sob a lei inegociável do Gerenciamento de Memória do Rust:

### 4.1. PT-1 (Higiene de RAM & Guilhotina de Processos)
A existência do PurgeGuard é justificada estritamente para impedir entropia:
1. Garantir que **nenhum** processo zumbi em background (como `oxlint`, linter estático infinito ou instâncias de `jcodemunch`) sobreviva ao ciclo de vida do repositório.
2. Garantir que o Ramdisk (que sequestra gigabytes reais da máquina do usuário) seja liberado ativamente após o processamento.

O PurgeGuard DEVE honrar o RAII (Resource Acquisition Is Initialization). Em vez de rodar métodos caóticos de limpeza estrutural e shells longos de demolição manual na função, o nó confia inteiramente em delegar a destruição pesada à implementação implícita ou explícita do Trait `Drop` dos seus parâmetros, guiando os objetos até o Fim de Escopo determinístico.

## 5. Critérios de Conclusão (Definition of Done - DoD)
- [ ] A interface da função força o consumo por valor das structs pesadas (`sandbox`, `ramdisk`).
- [ ] O retorno é nativamente `()` e o método não faz throw/bubble de falhas via operador `?`.
- [ ] A mecânica principal é engatilhar a liberação de recursos nativa do Rust (Drop timing) somada a qualquer chamada de unmount pendente opcional do OS, empacotadas de forma segura em blocos `match` ou `let _ = ...`.
- [ ] Zero presença de comandos de pânico (`panic!`, `.expect()`, `.unwrap()`) em produção.
- [ ] Geração de Log Warn (Ghost Telemetry) assegurada para os cenários onde o SO negue a destruição no prazo exigido.
