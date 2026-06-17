# Debug Session: marknative-goose-gaps

Status: OPEN

Symptom:
- `https://github.com/Liyown/Marknative` tem furos na `MASTER_SOLUTIONS` em:
  - `indicacao_otimista_canibalizacao`
  - `ouro_a_extrair`
  - `deep_pattern`
  - `transplantable_core`
  - `logic_math_heuristic`
  - `real_structural_problem`
  - `integracao_papel_exato`
  - `categoria_nuance_tecnica`
- `https://github.com/aaif-goose/goose` tem furo em:
  - `indicacao_otimista_canibalizacao`

Hypotheses:
1. Os campos faltantes já estão vazios em `repo_heuristics`, então o furo nasce antes da F4.
2. O checkpoint da F3 está pulando cedo com payload parcial persistido.
3. A F4 escreveu parcialmente ou ignorou colunas específicas no payload dinâmico.
4. `goose` sofre um caso de merge seletivo em apenas um campo do bloco 2A.
5. `Marknative` e `goose` compartilham a mesma causa raiz, mas em estágios diferentes da F3.

Plan:
1. Inspecionar `repositorios` e `repo_heuristics` para os dois repos.
2. Mapear se os campos faltantes estão vazios na L2 ou só na planilha.
3. Determinar se a causa é geração parcial, checkpoint falso ou injeção parcial.
4. Regenerar repo a repo e reinjetar apenas se a evidência apontar necessidade.

Findings:
- `repositorios` estava íntegro para ambos:
  - `Liyown/Marknative` -> `CONCLUIDO`, versão `v0.5.0`
  - `aaif-goose/goose` -> `CONCLUIDO`, versão `v1.37.0`
- Os campos faltantes estavam vazios também em `repo_heuristics`, então o problema não era exclusivo do Sheets.
- Ambos estavam com `status_fase = FASE_4_SHEETS_UPDATED` e `status_atualizacao = CONCLUIDO_AGUARDANDO`.

Root Cause A - Marknative:
- O payload persistido tinha apenas bloco 1 pronto; o bloco 2A inteiro estava vazio.
- A correção anterior do checkpoint terminal (`status_stage=5` vs `content_stage<5`) foi suficiente.
- Reexecução:
  - `cargo run --features tauri-app --bin f3_synthesizer_cli -- --repo Liyown/Marknative --e2e-full`
- Evidência:
  - `checkpoint terminal invalidado ... stage_from_status=5 stage_from_content=1`
  - `Bloco 2A concluído`
  - escrita confirmada no Sheets linha `147`

Root Cause B - goose:
- Caso mais fino: apenas `indicacao_otimista_canibalizacao` estava vazia.
- O campo pertence ao `BLOCK2A_FIELDS_COLUMNS`, mas o cálculo de `block2a_ok` não o considerava, então a F3 tratava o bloco 2A como completo.
- Instrumentação temporária confirmou em runtime:
  - `block2a_ok=true`
  - `indicacao_empty=true`
  - demais campos do bloco 2A preenchidos
- Correção aplicada em `src-tauri/src/cognition/synthesizer.rs`:
  - criação de `is_block2a_complete(row)`
  - inclusão de todos os campos narrativos do bloco 2A no critério de completude:
    - `indicacao_otimista_canibalizacao`
    - `ouro_a_extrair`
    - `deep_pattern`
    - `transplantable_core`
    - `logic_math_heuristic`
    - `real_structural_problem`
    - `categoria_nuance_tecnica`
    - `integracao_papel_exato`
- Teste adicionado:
  - `block2a_requires_indicacao_and_all_narrative_fields`
- Reexecução:
  - `cargo run --features tauri-app --bin f3_synthesizer_cli -- --repo aaif-goose/goose --e2e-full`
- Evidência pós-fix:
  - `checkpoint terminal invalidado ... stage_from_status=5 stage_from_content=1`
  - `Bloco 2A concluído`
  - `indicacao_otimista_canibalizacao` preenchida no SQLite
  - escrita confirmada no Sheets linha `417`
