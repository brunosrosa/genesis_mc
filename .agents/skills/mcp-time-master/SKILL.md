---
name: mcp-time-master
description: O Relógio do SODA. Ancoragem cronológica absoluta. Força a IA a consultar o fuso horário e a data real antes de registrar logs, criar branches temporais ou atualizar o Kanban, erradicando alucinações temporais.
triggers: ["mcp-time-master", "ver hora", "que dia é hoje", "data atual", "ancoragem temporal", "agendar", "timestamp", "fuso horário"]
---

# Skill: MCP Time Master (Ancoragem Cronológica)

## Goal
Atuar como a âncora de realidade temporal para o agente dentro do Antigravity IDE. Modelos de linguagem sofrem de desconhecimento temporal endêmico e alucinam datas baseadas em seus dados de treinamento. O objetivo inegociável desta habilidade é garantir que a IA nunca invente ou presuma o dia e a hora atuais ao nomear arquivos, escrever logs no Frontmatter, criar *branches* de *Snapshot* (GitOps) ou atualizar metadados no Kanban. 

## Instructions
Sempre que uma tarefa exigir a marcação de tempo, criação de históricos, ou quando você precisar saber o contexto cronológico exato da sua execução, você DEVE utilizar exclusivamente o MCP `time_server` seguindo estas regras:

1. **Proibição de Alucinação Temporal:** Você está expressamente PROIBIDO de presumir a data e a hora com base no contexto do chat ou em respostas anteriores. O tempo flui e o seu contexto fica obsoleto.
2. **Consulta Precisa:** Invoque OBRIGATORIAMENTE a ferramenta `time_server_get_current_time` exposta pelo Gateway. 
3. **Parâmetro de Fuso Horário:** Se o usuário não especificar, consulte a hora local fornecendo o fuso horário correto do ambiente (ex: "America/Sao_Paulo" ou UTC).
4. **Ancoragem de Artefatos:** Após receber o *timestamp* exato:
   - Use-o para nomear *branches* temporárias (ex: `git checkout -b backup_$(date +%s)`).
   - Use-o para assinar atualizações no YAML Frontmatter dos documentos (ex: `last_updated: "2026-04-17T14:30:00"`).
   - Use-o para entender prazos estipulados em *Issues* do GitHub MCP.

## Constraints
* **ZERO COMANDOS DE SHELL FRÁGEIS:** Não tente usar comandos de sistema como `date` via terminal interativo, pois a formatação varia entre Windows e Linux. Confie unicamente no retorno JSON estruturado da ferramenta MCP.
* **SILÊNCIO OPERACIONAL:** Você não precisa avisar ao usuário "Vou verificar que horas são". Faça a consulta no *background*, absorva a resposta e execute a tarefa entregando o artefato já datado corretamente.

## Examples

**Entrada do Usuário:** 
"SODA, faça um snapshot do nosso repositório agora usando a nossa skill de Subrepo, precisamos salvar o estado de hoje."

**Ação do Agente:**
1. O agente não adivinha a data.
2. Ele invoca silenciosamente a ferramenta `time_server_get_current_time` com o timezone local.
3. O MCP retorna que hoje é `2026-04-17 15:45:00`.
4. O agente invoca a skill de Subrepo e cria a branch com a nomenclatura precisa ancorada no tempo real obtido.