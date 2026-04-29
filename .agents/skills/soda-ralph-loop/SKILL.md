---
name: soda-ralph-loop
description: O Motor Implacável de Resiliência do Antigravity IDE. Roda 'cargo test' e 'cargo check' exigindo pureza total (-D warnings). Aplica a Morte e Renascimento (Context Purge) a cada iteração para zerar a VRAM. Kill-switch rígido de 3 tentativas.
triggers: ["soda-ralph-loop", "testar código", "rodar testes", "corrigir erro", "loop de compilação", "ralph loop", "auto-fix", "debug"]
---

### skill: SODA Ralph Loop (O Motor Implacável de Resiliência V3.0)

#### Goal
Garantir a integridade mecânica absoluta do código gerado no Antigravity IDE, agindo como o disjuntor entre a estocástica do LLM e a matemática do Rust. O objetivo inegociável é automatizar o fluxo "Tentativa-e-Erro" no terminal local sob a regra de Tolerância Zero (-D warnings), forçando você (o Agente) a aniquilar falhas lógicas, violações de Borrow Checker e "dead codes", com limpeza estrita de VRAM (Context Purge) a cada iteração, até o Exit Code 0 ou o Kill-Switch.

#### Instructions (Execução Coercitiva no Terminal Local)
Sempre que finalizar a escrita de código sob o SDD, ou quando a compilação falhar, execute esta Máquina de Estados:

1. **A Invocação Tolerância Zero (Validação Estrita):**
   * Abra o terminal do Antigravity IDE e execute os testes/compilação forçando a formatação curta e a promoção de avisos a erros.
   * Comando OBRIGATÓRIO: `cargo check --message-format=short -D warnings > tmp_ralph.log 2>&1` ou `cargo test -q > tmp_ralph.log 2>&1`.

2. **Extração Espartana (Proteção de VRAM):**
   * Leia o arquivo `tmp_ralph.log`. Graças ao formato curto, o erro estará isolado em linhas específicas (ex: `error[E0...]`).
   * Absorva apenas o identificador do erro e a linha apontada.

3. **Morte e Renascimento (Context Purge Obrigatório):**
   * **Lei da Amnésia Iterativa:** Para não estourar a memória (VRAM Limit), você DEVE esquecer (descartar mentalmente) o código exato e a justificativa da sua tentativa anterior. Mantenha em mente APENAS a regra quebrada e a nova solução.
   * **Proibição de Voodoo:** Não contorne o Borrow Checker com `.clone()` cego ou `unsafe {}`. Respeite a arquitetura Zero-Copy.
   * Aplique a mutação no arquivo fonte e retorne OBRIGATORIAMENTE para o Passo 1.

4. **O Kill-Switch (Circuit Breaker de Segurança):**
   * Você possui um limite inquebrável de **3 (TRÊS) ITERAÇÕES MÁXIMAS**.
   * Se não obtiver Exit Code 0 na 3ª tentativa, o loop MORRE.
   * Exclua o arquivo `tmp_ralph.log`, interrompa a execução e relate no Canvas: a falha fatal, a restrição de memória/sintaxe intransponível, e exija a intervenção do Arquiteto (HITL).

5. **A Vitória Silenciosa (Exit Code 0):**
   * Se o terminal retornar sucesso sem warnings (Exit Code 0), a iteração está validada.
   * Exclua fisicamente o log (`rm tmp_ralph.log` ou `del tmp_ralph.log`).
   * Notifique no chat espartanamente: `-> Ralph Loop concluído. Compilação Pura e Testes OK (Exit Code 0).`

#### Constraints
* **O COMPILADOR NÃO NEGOCIA:** Nunca tente explicar no chat por que o código "deveria" funcionar se o `cargo check` diz que falhou. Abrace a falha e corrija.
* **SILÊNCIO OPERACIONAL:** O usuário não quer ver o seu sofrimento. Tente consertar sozinho em *background*. Comunique-se apenas quando o Kill-Switch disparar ou quando vencer.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável.

#### Examples
**Entrada do Usuário:** "Módulo de SQLite inserido. Roda o Ralph Loop pra checar."

**Ação do Agente:**
1. Roda: `cargo check --message-format=short -D warnings > tmp_ralph.log 2>&1`.
2. Lê o arquivo e encontra: `error: unused variable: 'db_path'`. (Iteração 1).
3. O agente faz o *Context Purge* (esquece a justificativa antiga), remove a variável ociosa ou a implementa corretamente.
4. Roda novamente (Iteração 2). O terminal devolve Exit Code 0.
5. Deleta o arquivo temporário `tmp_ralph.log`.
6. Envia no Canvas: `-> Ralph Loop concluído em 2 iterações. Código puro e sem warnings (Exit Code 0).`
