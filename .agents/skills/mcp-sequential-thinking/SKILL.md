---
name: mcp-sequential-thinking
description: O Freio de Mão Cognitivo do Antigravity IDE. Delega o "Overthinking" ao MCP sequentialthinking com Hard-Limit de 5 iterações. Força a dialética de destruição (falsificação interna) e transita silenciosamente para o @soda-sdd, protegendo a VRAM e o orçamento (FinOps).
triggers: ["mcp-sequential-thinking", "raciocinar passo a passo", "pensar", "analisar problema", "planejar refatoração", "sequential thinking", "desdobrar lógica"]
---

### skill: MCP Sequential Thinking (O Freio de Mão e Dialética Interna)

#### Goal
Atuar como o regulador de cadência cognitiva e filtro de viabilidade do agente dentro do Antigravity IDE. O objetivo inegociável é erradicar o *Vibe Coding* e o Monólogo de Confirmação. Para proteger o orçamento de tokens (FinOps) e a VRAM local, o agente DEVE delegar o raciocínio estrutural ao servidor MCP, aplicando uma dialética interna de falsificação (onde o pensamento subsequente tenta destruir a premissa anterior) sob um teto estrito de iterações, antes de acionar a "Lei de Ferro" da codificação.

#### Instructions
Sempre que se deparar com uma arquitetura nova, um bug complexo, ou quando for instruído a "planejar/raciocinar", execute esta máquina de estados:

1. **A Trava de Geração (Zero-Code):** 
   * Você está expressamente PROIBIDO de gerar código textual na sua primeira resposta.

2. **Invocação MCP (Firewall Compliance):** 
   * Acione OBRIGATORIAMENTE a ferramenta de nome exato `sequentialthinking` (respeitando o filtro L7 CEL do Gateway).

3. **A Dialética de Destruição (Max 5 Iterações):**
   * Emita os pensamentos de forma iterativa, mas aplique a mecânica de Falsificação Coercitiva:
     * *Thought 1 (Tese):* Definição da solução instintiva inicial.
     * *Thought 2 (Antítese):* Tente DESTRUIR a solução inicial usando as leis do SODA (ex: Isso usa Node.js? Isso rompe os 6GB de VRAM? É Zero-Copy?).
     * *Thought 3 (Síntese):* Reajuste da arquitetura absorvendo a correção do Thought 2 (`isRevision: true`).
     * *Thought 4 (Validação):* Verificação final das restrições.
   * **HARD-LIMIT (FINOPS):** Você tem um orçamento máximo de 5 pensamentos (`thoughtNumber: 5`). No 5º pensamento, você é OBRIGADO a finalizar o loop enviando `nextThoughtNeeded: false`.

4. **Trabalho Invisível (Proteção de VRAM):**
   * O usuário NÃO deve ler o seu processo iterativo no Canvas. A bagunça do raciocínio fica restrita aos logs de background do servidor MCP.

5. **O Engate Operacional (Handoff para SDD):**
   * Ao finalizar, exiba no chat do Antigravity IDE APENAS a **Conclusão Destilada** (A síntese arquitetural nua e crua).
   * Emende imediatamente a invocação conceitual da "Lei de Ferro": pergunte ao Arquiteto Humano se você tem autorização para invocar o `@soda-sdd` e abrir o *Shadow Workspace* para iniciar o `docs/design.md`.

#### Constraints
* **PROIBIÇÃO DE OVERTHINKING:** A ultrapassagem do Hard-Limit de 5 iterações é uma violação severa de FinOps. Pense de forma concisa e redutiva.
* **NOMENCLATURA ESTRITA:** O firewall bloqueia prefixos inventados. O nome da ferramenta é EXATAMENTE `sequentialthinking`.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável.

#### Examples
**Entrada do Usuário:** "SODA, o lance de transmutação de bytes no IPC tá engasgando. Pensa numa forma de resolver isso em Zero-Copy absoluto."

**Ação do Agente:**
1. (Silenciosamente) Invoca `sequentialthinking`.
2. Thought 1: "Vamos passar o ArrayBuffer via JSON-RPC."
3. Thought 2: "FALHA. O JSON-RPC vai serializar o ArrayBuffer em Base64, estourando a VRAM e ativando o Garbage Collector do V8. Isso viola as Leis Duras."
4. Thought 3 (`isRevision: true`): "Ajuste: Devemos usar os Canais Binários nativos do Tauri v2 com *Transferable Objects*."
5. Thought 4: "Validação: Isso atinge a latência de <5ms sem GC. Concluído." (`nextThoughtNeeded: false`).
6. Responde no Canvas: *"Raciocínio concluído e validado contra VRAM. A solução é adotar canais binários nativos do Tauri v2 acoplados a Transferable Objects. Tenho autorização para invocar o @soda-sdd e criar o Shadow Workspace para materializar isso no design.md?"*