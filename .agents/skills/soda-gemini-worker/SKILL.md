---
name: soda-gemini-worker
description: O Códice Mestre de FinOps e Cloud Brain. Delega leituras massivas (>1M tokens) para a CLI do Gemini em background. Unifica STDOUT Piping (para proteger a VRAM), Jitter Anti-Ban, Plan Mode, injeção de Doutrina via GEMINI_SYSTEM_MD e Compressão de Contexto (/compress).
triggers: ["soda-gemini-worker", "ler repositório inteiro", "refatoração massiva", "chamar gemini", "usar worker da nuvem", "heavy duty", "subscription hacking", "finops", "cloud brain"]
---

### skill: SODA Gemini Worker (O Códice Mestre FinOps V3.0)

#### Goal
Atuar como a ponte de FinOps e força bruta cognitiva do SODA. Sua missão é proteger os 6GB de VRAM locais, delegando a *leitura e planejamento* de ecossistemas massivos para a nuvem via Gemini CLI. Você atua como o gestor do "Cloud Brain": o Gemini planeja a arquitetura (DAG), mas NUNCA executa a edição bruta dos arquivos. Você deve aplicar rigorosamente a proteção contra banimentos (Jitter), proteção de memória local (STDOUT Piping), submissão de doutrina (GEMINI_SYSTEM_MD) e compressão de contexto para não falir o usuário.

#### Instructions
Sempre que for exigido analisar arquiteturas inteiras ou planejar grandes refatorações (>100k tokens), você DEVE orquestrar a seguinte Máquina de Estados:

1. **A Lei da Injeção de Doutrina (`GEMINI_SYSTEM_MD`):**
   * O Gemini possui um viés perigoso de "querer reescrever tudo". Antes de invocar a CLI, você DEVE sobrescrever o comportamento padrão exportando a variável de ambiente: 
   * `export GEMINI_SYSTEM_MD="Aja como SODA Core. NUNCA reescreva arquivos inteiros. Crie apenas planos cirúrgicos focados em AST. Zero Vibe Coding."`

2. **Compressão Temporal (O Fim do Token Bloat):**
   * Se for analisar a base inteira, NUNCA submeta a pasta `src/` recursivamente múltiplas vezes. 
   * Execute o comando de compressão primeiro: `gemini /compress "@src/" > .agents/tmp/CONTEXT_POINTERS.md`.
   * Use o arquivo de ponteiros gerado para todas as consultas de planejamento subsequentes.

3. **Jitter de Emulação Humana (Proteção Anti-Ban):**
   * Os provedores rastreiam automações via CLI. Antes de acionar o subprocesso, injete OBRIGATORIAMENTE um atraso estocástico (ex: `sleep $((3 + RANDOM % 5))`).

4. **A Lei do Cloud Brain (Plan Mode Obrigatório):**
   * A nuvem atua estritamente como "Arquiteto Consultor". Você DEVE forçar o modo de planejamento para que a IA não tente alterar arquivos do host.
   * **Sintaxe Obrigatória:** Use a flag `/plan` antes do prompt.

5. **Blindagem de VRAM e STDOUT Piping (Inegociável):**
   * É EXTREMAMENTE PROIBIDO executar a CLI permitindo que ela imprima a resposta massiva diretamente no seu terminal do IDE (isso causará OOM local).
   * Você DEVE redirecionar silenciosamente a saída bruta para o disco (adicionando `> .agents/tmp/gemini_output.md`).
   * Após a conclusão em *background*, leia cirurgicamente o arquivo, extraia as tarefas (DAG) e apague o lixo excedente.

#### Constraints
* **MIGRAÇÃO DE API KEY vs CONTA SILO:** Alerte o usuário para configurar a CLI estritamente com uma API Key em modo Batch ($0.075/1M tokens) para operações massivas, evitando banimentos (Rate Limits) das assinaturas "Premium" de consumidor (100 reqs/5h).
* **ZERO EDIÇÃO DIRETA:** A CLI do Gemini NUNCA altera o código do usuário. É o Agente Antigravity local que aplica as mudanças do plano em um *Shadow Workspace*.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` no topo desta skill é inegociável.

#### Examples
**Entrada do Usuário:** "SODA, envia a pasta src/ (que tem uns 500k tokens) pro Gemini descobrir o gargalo do Zero-Copy IPC no Tauri. Me traga só o plano estruturado."

**Ação do Agente:**
1. O agente garante a existência do diretório `.agents/tmp/`.
2. Injeta a doutrina do sistema e realiza a compressão silenciosa: 
   `export GEMINI_SYSTEM_MD="Zero Vibe Coding. Responda apenas com o DAG atômico do problema." && gemini /compress "@src/" > .agents/tmp/CONTEXT_POINTERS.md`
3. Constrói o comando de planejamento final encapsulado com **Jitter**, **Plan Mode** e **STDOUT Piping**:
   `sleep 4 && gemini /plan "@.agents/tmp/CONTEXT_POINTERS.md Analise o tráfego IPC e gere tarefas de refatoração" > .agents/tmp/gemini_ipc_plan.md`
4. O subprocesso executa e morre em background.
5. O agente local (Antigravity) lê o arquivo temporário, ignora o ruído, e projeta as tarefas no Canvas do usuário.