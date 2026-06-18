# ARCH Reflection Report - Fase 3 e 4

Data: 2026-06-17
Escopo: auditoria do motor Rust da F3/F4, contrato JSON Schema/SGR, persistencia SQLite e espelho Sheets.

## 1. Arquivos Inspecionados

- `src-tauri/src/cognition/synthesizer.rs`
- `src-tauri/src/bin/f3_synthesizer_cli.rs`
- `src-tauri/src/persist/ssot_injector.rs`
- `docs/specs/MASTER_SOLUTIONS_DIC.csv`
- `docs/specs/DATABASE_SCHEMA_DIC.csv`
- `_FEEDBACK.md`

## 2. Visao de Facil Digestao

### 2.1 Quem governa as saidas da IA hoje

Nao ha Pydantic no runtime Rust. O papel equivalente esta repartido entre:

1. `response_format_for_block()` em `f3_synthesizer_cli.rs`
   - Monta o JSON Schema estrito enviado ao formatador.
   - Define `type`, `required`, `enum`, `maxLength`, `additionalProperties=false`.
   - E a primeira muralha SGR.

2. Structs Serde em `synthesizer.rs`
   - `Block1Fields`
   - `Block2NarrativeFields`
   - `Block2MatrixFields`
   - `Block3Fields`
   - `Block4Fields`
   - `BlockResponse<T>`
   - Elas fazem o parse tipado e, em varios casos, o saneamento posterior.

3. `MasterSolutionsRow` em `synthesizer.rs`
   - E a struct agregadora/SSOT da linha final.
   - Governa a ordem interna usada para persistencia local e montagem da linha de saida.

4. `ensure_repo_heuristics_schema()` em `ssot_injector.rs`
   - Congela a ordem fisica do SQLite em `repo_heuristics`.
   - E a referencia fria da tabela L2.

5. `MASTER_SOLUTIONS_CANONICAL_COLUMNS` em `synthesizer.rs`
   - Congela a ordem canonica da linha de planilha.
   - E a referencia usada por `prepare_batch_payload_dynamic()` para o binding tardio no Sheets.

### 2.2 Achado critico de arquitetura

Existe drift entre o contrato do schema e a expectativa do runtime:

- O prompt e o executor da F3 assumem envelope com `fields + justifications`.
- Porem `response_format_for_block()` atualmente usa:
  - `block == 3` -> `envelope_fields_only(fields_schema)`
  - `demais blocos` -> `envelope_with_justifications(fields_schema)`
- Resultado: o bloco 3 nao exige `justifications` no schema, apesar do runtime da F3 tratar justificativas como parte obrigatoria da avaliacao e da logica anti-homogeneizacao.

Este ponto deve ser fechado no Passo 3.

## 3. Structs que Governam a Saida Atual

### 3.1 Estruturas narrativas

- `Block1Fields`
  - `proposta_original_resumo`
  - `declared_description_ptbr`
  - `visao_do_enxame`
  - `justificativa_decisao`
  - `executive_verdict`
  - `risco_principal`
  - `risco_linha_vermelha`
  - `observacoes`

- `Block2NarrativeFields`
  - `indicacao_otimista_canibalizacao`
  - `ouro_a_extrair`
  - `deep_pattern`
  - `transplantable_core`
  - `logic_math_heuristic`
  - `real_structural_problem`
  - `categoria_nuance_tecnica`
  - `integracao_papel_exato`

- `Block2MatrixFields`
  - `must_components_prod_ux`
  - `must_components_arq`
  - `must_components_ops`
  - `detected_toxic_deps`
  - `do_not_absorb`
  - `where_ai_should_not_enter`

### 3.2 Estruturas categoricas/enums

- `Block3Fields`
  - `classificacao_terminal`
  - `acao_de_canibalizacao`
  - `categoria_arquitetural`
  - `horizonte_extracao`
  - `tipo_integracao`
  - `capability_nature_primary`
  - `architectural_topology`
  - `temporal_stability`
  - `bare_metal_fit`
  - `extractability_level`
  - `runtime_sovereignty_fit`
  - `local_first_fit`
  - `adoptability_level`
  - `longitudinal_sustainability`
  - `maintenance_burden`
  - `onboarding_friction`
  - `observability_operational`
  - `recoverability_level`
  - `degradation_behavior`
  - `curation_burden`
  - `evolution_cost`
  - `operability_level`
  - `abandonment_risk`
  - `time_to_first_clear_value`
  - `imperfection_tolerance`
  - `entropy_risk`
  - `design_misuse_risk`
  - `intrinsic_ethics_risk`
  - `discipline_dependency`
  - `regulatory_risk`

- `Block4Fields`
  - `score_philosophical_fit`
  - `score_bare_metal_fit`
  - `score_architectural_extractability`
  - `score_operability`
  - `score_creep_risk`
  - `score_runtime_sovereignty`
  - `score_model_logic_value`
  - `score_ethics_safety`
  - `score_intrinsic_risk`

## 4. Onde Estao as Amarras de Tamanho / Truncagem

### 4.1 JSON Schema do formatador

Arquivo: `src-tauri/src/bin/f3_synthesizer_cli.rs`

- `string_schema(max_len)` aplica `maxLength` em todo campo string do schema.
- Bloco 1:
  - `proposta_original_resumo` -> `maxLength = 5000`
  - `declared_description_ptbr` -> `5000`
  - `visao_do_enxame` -> `5000`
  - `justificativa_decisao` -> `5000`
  - `executive_verdict` -> `5000`
  - `risco_principal` -> `5000`
  - `risco_linha_vermelha` -> `5000`
  - `observacoes` -> `5000`
- Bloco 2A:
  - `indicacao_otimista_canibalizacao` -> `5000`
  - `ouro_a_extrair` -> `5000`
  - `deep_pattern` -> `5000`
  - `transplantable_core` -> `5000`
  - `logic_math_heuristic` -> `5000`
  - `real_structural_problem` -> `5000`
  - `categoria_nuance_tecnica` -> `2000`
  - `integracao_papel_exato` -> `2000`
- Bloco 2B:
  - arrays com `maxItems <= 8`
  - cada item com `maxLength = 800`
- `justifications.additionalProperties` -> `maxLength = 5000`

### 4.2 Prompting que comprime a saida

Arquivo: `src-tauri/src/cognition/synthesizer.rs`

- Block 1:
  - `STYLE_BLOCK1: seja conciso (5 a 8 linhas por campo)`
  - Isso nao corta por schema, mas induz compressao semantica.
- Block 3:
  - `LIMITS_BLOCK3: cada valor string ... no maximo 180 caracteres`
  - Esta e uma amarra dura e explicita.

### 4.3 Truncagem do contexto de entrada

Arquivo: `src-tauri/src/cognition/synthesizer.rs`

No `compact_context_for_block()` ha poda do contexto antes de enviar ao modelo:

- `lente_a_sentido_prod_ux` -> 2200 chars
- `lente_b_estrutura_arq` -> 2200 chars
- `lente_c_realidade_ops` -> 2200 chars
- `proposta_original_resumo` -> 1200 chars
- `indicacao_otimista_canibalizacao` -> 1800 chars

### 4.4 Conclusao objetiva sobre os 4 campos narrativos citados no feedback

As colunas abaixo NAO estao hoje sob `maxLength` agressivo no schema da F3/F4:

- `visao_do_enxame`
- `executive_verdict`
- `risco_principal`
- `risco_linha_vermelha`

Hoje elas estao em `5000` no bloco 1. Logo, o truncamento percebido nelas parece mais compativel com:

- compressao induzida pelo prompt (`seja conciso`)
- resposta curta do modelo
- ou perda de densidade semantica por contexto resumido antes da inferencia

Ou seja: ha amarras reais no motor, mas nao precisamente via `maxLength` desses 4 campos.

## 5. Ordem Atual dos Campos

### 5.1 Ordem do schema por bloco

#### Block 1

1. `proposta_original_resumo`
2. `declared_description_ptbr`
3. `visao_do_enxame`
4. `justificativa_decisao`
5. `executive_verdict`
6. `risco_principal`
7. `risco_linha_vermelha`
8. `observacoes`

#### Block 2A

1. `indicacao_otimista_canibalizacao`
2. `ouro_a_extrair`
3. `deep_pattern`
4. `transplantable_core`
5. `logic_math_heuristic`
6. `real_structural_problem`
7. `categoria_nuance_tecnica`
8. `integracao_papel_exato`

#### Block 3

1. `classificacao_terminal`
2. `acao_de_canibalizacao`
3. `categoria_arquitetural`
4. `horizonte_extracao`
5. `tipo_integracao`
6. `capability_nature_primary`
7. `architectural_topology`
8. `temporal_stability`
9. `bare_metal_fit`
10. `extractability_level`
11. `runtime_sovereignty_fit`
12. `local_first_fit`
13. `adoptability_level`
14. `longitudinal_sustainability`
15. `maintenance_burden`
16. `onboarding_friction`
17. `observability_operational`
18. `recoverability_level`
19. `degradation_behavior`
20. `curation_burden`
21. `evolution_cost`
22. `operability_level`
23. `abandonment_risk`
24. `time_to_first_clear_value`
25. `imperfection_tolerance`
26. `entropy_risk`
27. `design_misuse_risk`
28. `intrinsic_ethics_risk`
29. `discipline_dependency`
30. `regulatory_risk`

#### Block 4

1. `score_philosophical_fit`
2. `score_bare_metal_fit`
3. `score_architectural_extractability`
4. `score_operability`
5. `score_creep_risk`
6. `score_runtime_sovereignty`
7. `score_model_logic_value`
8. `score_ethics_safety`
9. `score_intrinsic_risk`

### 5.2 Ordem canonica no Sheets (SSOT)

Arquivo: `MASTER_SOLUTIONS_CANONICAL_COLUMNS`

Ponto importante:

- A ordem canonica da planilha nao segue uma disciplina SGR pura.
- Exemplos de desalinhamento:
  - `categoria_arquitetural` aparece muito cedo, antes do corpo narrativo.
  - `classificacao_terminal` entra antes de varios pares argumentativos de risco e observacao.
  - `acao_de_canibalizacao` aparece antes de alguns campos que explicam o por que da amputacao.

Isso nao quebra a escrita no Sheets, mas empurra a maquina para um layout menos favoravel ao raciocinio guiado por schema.

## 6. Declared Description - Estado Atual

Hoje a heuristica operacional encontrada foi:

1. tenta aproveitar seed ja persistida
2. se estiver vazio/unknown, tenta derivar a descricao do README via `derive_declared_description_from_readme()`

Nao encontrei, na F3/F4 auditada, uma heuristica priorizando explicitamente:

1. ABOUT do topo da pagina GitHub
2. README
3. titulo da pagina

Logo, o feedback do Arquiteto procede: essa prioridade ainda nao esta cristalizada no motor documentado.

## 7. Estado Atual do SQLite / Persistencia

Tabela principal da F3/F4:

- `repo_heuristics`

Pontos de ancoragem:

- `ensure_repo_heuristics_schema()` define a ordem fria da tabela.
- `upsert_repo_heuristics_row_internal()` faz o `INSERT OR REPLACE` usando a ordem fisica da tabela.
- `prepare_batch_payload_dynamic()` usa `MASTER_SOLUTIONS_CANONICAL_COLUMNS` e `to_sheet_row()` para montar o payload do Sheets.

Observacao:

- A ordem da struct `MasterSolutionsRow`
- a ordem do SQLite
- e a ordem canonica do Sheets

nao sao identicas. O sistema funciona porque ha mapeamento explicito, mas isso aumenta a superficie de drift quando o schema evolui.

## 8. Passo 3 - Estruturado, Ainda Nao Aplicado

Mudancas propostas para o crivo do Arquiteto:

1. Refatorar o contract builder de `response_format_for_block()` para:
   - endurecer os enums com o catalogo do `_FEEDBACK`
   - exigir `justifications` tambem no bloco 3
   - remover a mentalidade de enum legado/alias onde ela conflitar com o SSOT novo

2. Refatorar `Block3Fields` e enums associados em `synthesizer.rs` para:
   - alinhar os enums canonicos novos
   - reduzir fallback silencioso em `Unknown`

3. Aplicar SGR explicito na ordem:
   - manter/forcar campos argumentativos antes dos enums correspondentes
   - revisar `BLOCK3_FIELDS_COLUMNS`
   - revisar, se aprovado, a ordem de `MASTER_SOLUTIONS_CANONICAL_COLUMNS` apenas se o impacto no Sheets for aceitavel

4. Revisar compressao textual:
   - retirar amarras artificiais onde houver `maxLength` desnecessario
   - revisar o texto `STYLE_BLOCK1: seja conciso`
   - preservar limite curto apenas onde o campo e categorico

5. Declarar oficialmente a heuristica de `declared_description` no codigo:
   - ABOUT GitHub primeiro
   - README como fallback
   - titulo/headline da pagina como fallback final

## 9. Resposta Curta para Decisao

- Passo 1: concluido.
- Passo 2: pronto para cristalizacao documental.
- Passo 3: mapeado e estruturado, mas ainda nao aplicado no Rust aguardando crivo humano.
