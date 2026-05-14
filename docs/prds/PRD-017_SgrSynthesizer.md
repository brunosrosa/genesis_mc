# PRD-017: SgrSynthesizer (N17 - Sintetizador SGR)

## 1. Visão Geral
O `SgrSynthesizer` atua como o funil de consolidação da Fase 3. Ele converte o caos argumentativo gerado na Fase 2 (Enxame Cognitivo) em dados matematicamente tipados e estritos. Ao invés de usar *prompting* tradicional e torcer para o modelo cuspir um JSON válido, o nó emprega a **Decodificação Restrita** (via máscara de gramática como `llguidance`), coagindo o modelo de baixo custo a preencher uma estrutura imutável.

## 2. Assinatura do Contrato

### Entrada
- `repo_id: String`: Chave do repositório (para rastreio e cruzamento com a base L2).
- `debate: SwarmDebate`: Struct puramente textual contendo os campos brutos lidos do banco:
  - `lente_a: String`
  - `lente_b: String`
  - `lente_c: String`

### Saída
- `Result<SgrPayload, SgrError>`: Sucesso exige o retorno de uma struct fortemente tipada (`SgrPayload`), onde cada campo reflete exatamente uma coluna crítica da base canônica (Master Solutions), com todos os tipos (Int, Enum, String) validados a nível de bit na memória do Rust.

## 3. A Lei do SGR (Schema-Guided Reasoning)
A Decodificação Restrita obriga o LLM a preencher os campos do JSON/Payload na exata ordem definida pelo *Schema*. O SODA utiliza isso como uma arma cognitiva.

**A Regra da Ordem:**
É obrigatório que a *Struct* exija os campos textuais argumentativos ANTES dos campos numéricos ou categóricos finais. O layout da máscara de geração obrigatoriamente segue o fluxo:
1. `visao_do_enxame` (Síntese holística dos debates).
2. `justificativa_decisao` (Defesa estruturada dos pesos).
3. `executive_verdict` (Sentença final do agente corporativo).
4. `score_bare_metal_fit` (Int/Enum gerado).
5. `score_final` (Int gerado).

*Fundamento Arquitetural:* Isso força o LLM a injetar seu raciocínio lógico no *KV Cache* (Chain of Thought embutido na serialização) antes de ser forçado a vomitar as notas estáticas, extinguindo alucinações prematuras.

## 4. Invariantes de Blindagem (Proibições Tóxicas)
- **PT-SGR-1 (Fobia de Médias Aritméticas):** É **TERMINANTEMENTE PROIBIDO** instruir a nuvem a calcular qualquer forma de média ponderada ou consenso democrático entre as Lentes para chegar ao número final. A instrução do prompt deve ser draconiana impondo o **Score Punitivo**:
  - Se o `SwarmDebate` apontar uso crônico de Node.js no backend, micro-VMs pesadas no núcleo de execução Bare-Metal ou dependências tóxicas irreparáveis...
  - A nota de `score_bare_metal_fit` recebe zero imediato e puxa o `score_final` fatalmente para baixo.
  - Não há negociação de médias; uma falha de "HardwareOps" ou "Stack Imutável SODA" atua como um Curto-Circuito Lógico na gravidade final do projeto.
