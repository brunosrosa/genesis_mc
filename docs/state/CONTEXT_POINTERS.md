# 🗺️ Mapeamento de Contexto SODA (Pointer Index)
Este documento é o índice mestre da arquitetura Genesis MC. **NÃO HÁ NECESSIDADE DE LER TODOS OS ARQUIVOS.** Use ferramentas de leitura de arquivo (ex: `get_file_contents` ou `read_file`) apenas no arquivo que corresponde ao domínio da sua tarefa atual.

## 📌 Documentos Ativos 
- `AGENTS.md` -> A Constituição do SO, restrições de Hardware (RTX 2060m) e Regras do usuário (2e/TDAH).
- `.agents/rules/tech-stack.md` -> A pilha tecnológica imutável (Rust, Tauri v2, Svelte 5, SQLite WAL).
- `.agents/rules/DESIGN.md` -> O Sistema de Design (Cyber-Purple, Zero Layout Shift, Tailwind v4).
- `./docs/milestones/PRD_MILESTONE_01.md` -> O escopo **exato e restrito** da tarefa atual que você deve executar.

## 🏛️ Arquitetura Profunda (docs/architecture/)
*Consulte apenas se precisar entender a fundo a teoria do sistema antes de codificar:*
- `./docs/architecture/manifesto_soda.md` -> Filosofia de produto e "Sovereign OS".
- `./docs/architecture/core_daemon.md` -> A lógica do Daemon em Rust e processos.
- `./docs/architecture/gateway_routing.md` -> Como o AgentGateway e o protocolo MCP funcionam.
- `./docs/architecture/inference_engine.md` -> A gestão de inferência híbrida na CPU/GPU (llama.cpp / llama_cpp_rs).
- `./docs/architecture/memory_system.md` -> Como a memória Tri-Partite (SQLite + LanceDB) é desenhada.

## ⚖️ Regras do Sistema (.agents/rules/)
*Consulte para garantir a metodologia de código:*
- `.agents/rules/governance_bmad.md` -> O fluxo OBRIGATÓRIO de Spec-Driven Development e Shadow Workspaces.

## 🛠️ Habilidades (.agents/skills/)
- Explore os arquivos `SKILL.md` nos subdiretórios para entender as ferramentas e metadados disponíveis antes de invocá-las.