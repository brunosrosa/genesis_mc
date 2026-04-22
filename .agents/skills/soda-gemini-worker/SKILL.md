---
name: soda-gemini-worker
description: Orquestrador de FinOps (Subscription Hacking). Delega tarefas de leitura e refatoração massiva (1M+ tokens) para a CLI do Gemini em background. Aplica Jitter de emulação humana e Plan Mode para evitar banimentos e alucinações.
triggers: ["soda-gemini-worker", "ler repositório inteiro", "refatoração massiva", "chamar gemini", "usar worker da nuvem", "heavy duty", "subscription hacking", "finops"]
---

# Skill: SODA Gemini Worker (Heavy Duty FinOps)

## Goal
Atuar como a ponte econômica e de força bruta (Subscription Hacking) do Genesis MC [3]. O objetivo inegociável desta habilidade é proteger os parcos 6GB de VRAM do hardware local delegando tarefas com contexto massivo (centenas de milhares de tokens) para a infraestrutura do Gemini CLI [4]. Em vez de consumir APIs pagas por token (que gerariam o "Inference Bill Shock"), você usará subprocessos locais do `gemini` CLI atrelados à assinatura Google One AI Premium do usuário [5, 6]. Para evitar banimentos por violação de Termos de Serviço (ToS) e *Overthinking* destrutivo do modelo, você DEVE forçar emulação de atrasos humanos (Jitter), operar em modos estritos de planejamento e garantir a integridade via Conta Silo [1, 2].

## Instructions
Quando o usuário exigir que você "analise o projeto todo", "mapeie a arquitetura", ou "refatore múltiplos arquivos", a VRAM local entrará em colapso se você tentar processar isso internamente. Você DEVE invocar esta habilidade executando a seguinte Máquina de Estados:

1. **A Tática WISC (Write, Isolate, Select, Compress):**
   - Não jogue logs de erro massivos e não filtrados diretamente para a CLI [2]. 
   - Isole a raiz do problema via *diffs* locais e crie um arquivo de contexto direcionado temporário para alimentar o Gemini de forma cirúrgica [2].

2. **Levantamento Aéreo (Headless / Macro Mode):**
   - Execute o Gemini CLI como um subprocesso no terminal, forçando o modo *headless* para leitura estática de múltiplos diretórios [2].
   - **Sintaxe Obrigatória:** Invoque `gemini -p "@src/ @docs/ Elabore o diagrama da lógica no arquivo temporário"` [2].

3. **Imposição do Plan Mode (O Fim do Overthinking):**
   - Para forçar a IA da nuvem a criar planos atômicos sem reescrever arquivos massivamente por conta própria (o que gera *Context Rot*), invoque-a estritamente no modo de planejamento [7-9].
   - **Sintaxe Obrigatória:** `gemini /plan "Esquematize a edição atômica em múltiplos passos e justifique com referências locais."` [2].

4. **Jitter de Emulação Humana (Proteção Anti-Ban):**
   - O provedor rastreia e bloqueia CLIs automatizadas sob suspeita de tráfego de bot (*Shadow Bans*) [1, 10]. 
   - Ao orquestrar múltiplos comandos sequenciais para a CLI, você DEVE injetar atrasos estocásticos usando comandos bash (ex: `sleep $((2 + RANDOM % 4))` ou similar na sua linguagem de orquestração) para emular a velocidade de digitação e leitura humana entre os *prompts* [1, 2].

5. **Consolidação em Background e Descarte:**
   - Quando a CLI do Gemini terminar a execução longa, absorva estritamente o *Output* textual/JSON gerado e encerre sumariamente o subprocesso para não manter processos zumbis poluindo a RAM do PC hospedeiro [2, 11].
   - Apresente o plano destilado no Canvas do Antigravity para aprovação do usuário.

## Constraints
* **CONTA SILO OBRIGATÓRIA:** Se for o primeiro uso, alerte imperativamente o usuário: "Certifique-se de que a autenticação no Gemini CLI está utilizando uma Conta Silo (dedicada apenas para IA) e NUNCA o seu e-mail pessoal/primário. O risco de banimento por automação deve ter dano colateral zero." [1].
* **ZERO EDIÇÃO DIRETA NO DISCO:** A CLI do Gemini está terminantemente PROIBIDA de aplicar código diretamente nos seus arquivos-fonte [12]. Ela atua como um "Consultor" que cospe o código ou plano no terminal [12]. A aplicação das mudanças no disco deve ser feita por você (o Agente SODA local) utilizando a técnica de *Shadow Workspace* ou através do script `strict_enforcer.sh` [12, 13].

## Examples
**Entrada do Usuário:** 
"SODA, pega a nossa pasta `src/` inteira, que tem mais de 200 mil tokens, e pede pro Gemini achar o gargalo de concorrência no Tokio. Traz só o plano de ação."

**Ação do Agente:**
1. O agente alerta o usuário sobre a verificação da Conta Silo [1].
2. O agente constrói o comando com *Jitter*: `sleep 3 && gemini /plan "@src/ Analise a concorrência do Tokio e aponte os gargalos de MPSC em formato de lista estruturada."` [1, 2].
3. O subprocesso executa silenciosamente em *background* [14].
4. O agente intercepta a resposta da CLI, finaliza o processo do Gemini, e imprime o plano de ação validado no Canvas, aguardando ordem para iniciar a codificação local [2, 14].