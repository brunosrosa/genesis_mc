---
id: "ADR-041"
title: "ADR-041-Nomenclatura-Soberana-Servername-souls_mcp"
version: 1.0
status: Ativo_Inegociavel
epic: "Infraestrutura / Governanca"
amends: ["ADR-026"]
revoga_parcialmente: "ADR-026 §1 (nome de servidor atômico 'souls' passa a 'souls_mcp')"
description: "Emenda Constitucional 32/120 + Servername Soberano 'souls_mcp': institui o nome canonico do Agent Gateway e os tetos rigidos de 32 caracteres (nome) e 120 (descricao) para ferramentas MCP."
---

# ADR-041 — Nomenclatura Soberana & Servername `souls_mcp` (Emenda Constitucional 32/120)

## Status
Aceito (Ativo e Inegociavel). Emenda parcial da [ADR-026](docs/decisions/adrs/ADR-026-Nomenclatura-Semantica-Zero-Brand.md) (revoga §1 sobre o nome de servidor `souls` e fixa o nome canonico `souls_mcp`).

## Contexto
A ADR-026 instituiu a **Lei Zero-Brand** e fixou o servidor atomico `souls` para evitar nomes acoplados a marcas. Com a chegada do **Roteador Semantico ParetoBandit** e a previsao de 50+ ferramentas, a cerca perimetrica do Roteador (distincao nativo vs terceiro) precisa de um **radical unico e inequivoco** no servername. Alem disso, o crescimento das tools exige tetos dimensionais para impedir a hemorragia FinOps (cada caractere e token) e a quebra de clientes MCP com limites de exibicao < 60 chars.

A concaternacao tipica de clientes MCP (`<server>_<tool>` ou `<server>.<tool>`) demonstrou que `souls.get_ast` (servername `souls` + tool curta) consome **11 chars por chamada**, enquanto `souls_mcp.get_ast` consome **18 chars** mas entrega **cerca perimetrica no server** (1 lookup O(1) identifica tudo que e nativo) em vez de espalhada em 40 tools (canibalizadas ou futuras).

## Decisao (Emenda Constitucional 32/120 + Servername Soberano)

Fica **revogada parcialmente** a §1 da ADR-026 (que fixava o servername como `souls`) e ficam **instituidas** as seguintes Leis Duras:

1. **Servername Soberano:** O Agent Gateway exportado para os clientes MCP passa a se chamar estritamente **`souls_mcp`**. O nome antigo `souls` fica obsoleto.

2. **Cerca Perimetrica por Servername (Canibalizacao Cirurgica Preservada):** Toda ferramenta dentro de `souls_mcp.*` e, por construcao, **nativa, local-first e segura do SODA**. O Roteador Semantico distingue nativo de terceiro por **1 lookup O(1) no servername**, em vez de parsear prefixos em N tools.

3. **Preservacao da Canibalizacao:** As ferramentas ja canibalizadas (`get_ast`, `read`, `search`, `mem_search`, `core_think`, etc.) **mantem seus nomes curtos** (Zero-Brand da ADR-026 §2-4). Apenas ferramentas **novas** (Marcos futuros) podem opcionalmente adotar o prefixo `souls_` se houver ambiguidade semantica.

4. **Teto Rigido de Nome (≤ 32 caracteres):** Toda tool, presente ou futura, DEVE respeitar o teto de **32 caracteres** no campo `name` do `tools/list`. Validacao em tempo de compilacao (teste de snapshot) e em runtime (panic se violar).

5. **Teto Rigido de Descricao (≤ 120 caracteres):** Toda descricao de tool DEVE respeitar o teto de **120 caracteres**, ser **seca, tecnica, informativa e livre de prosa ou floreios de marketing** ("slop" e vetado).

6. **Reserva de Namespace:** Tools de terceiros ou APIs online NAO podem ser hospedadas sob o servername `souls_mcp`. Devem usar servernames proprios (`brave`, `actual_budget`, etc.).

7. **Backward Compat de Servername:** Durante a janela de transicao (1 release), clientes que consumem `souls.*` continuam funcionando via alias no `gateway-config.yaml` (target dual: `souls` → `souls_mcp_server.exe`, `souls_mcp` → mesmo binario). Apos a janela, alias `souls` e removido.

## Consequencias
* **Imunidade a Colisao de Namespace:** `souls_mcp.*` e a cerca perimetrica; nenhum terceiro pode injetar ferramentas nativas.
* **Canibalizacao Cirurgica Preservada:** Tools ja existentes (`get_ast`, `mem_search`, `core_think`) nao sofrem churn. Skills e docs nao precisam de Search and Replace em 90+ ocorrencias.
* **FinOps Controlado:** Render medio passa de `souls.get_ast` (11) para `souls_mcp.get_ast` (18) = +7 chars/chamada. Custo compensado pela **reducao de 1 lookup O(1)** no Roteador (em vez de parsear prefixo em 40 tools).
* **Tolerancia a Limite 60 chars MCP:** Servername `souls_mcp` (9) + ponto (1) + toolname (≤32) + ponto (1) = ate **43 chars**, abaixo do limite 60 com margem de 17.
* **Tetos 32/120:** Cobertura retroativa via teste de snapshot. Tools existentes em `tools/list` (40) que violem tetos serao flagged pelo teste.

## Restricoes Bare-Metal e Blast Radius
* **`src-tauri/src/bin/souls_mcp_server.rs`:** Trocar `serverInfo.name = "souls"` para `"souls_mcp"` (L129). Adicionar teste de snapshot `tools_list_respects_32_120_tetos` que valida tetos em runtime.
* **`gateway-config.yaml`:** Trocar `targets[0].name = "souls"` para `"souls_mcp"`. Adicionar target alias `souls` apontando para o mesmo binario (backward compat).
* **`.trae/rules/project_rules.md`:** Adicionar §5 (Lei do Servername Soberano + tetos 32/120).
* **`.agents/skills/**/SKILL.md`:** **Zero mutacao obrigatoria** (canibalizacao preservada). Search and Replace cirurgico apenas se alguma Skill invocar `souls.server` em vez de `souls_mcp.server`.
* **`docs/runtime/context_dumps/_YAML_AgentGateway_e_souls_mcp_server.rs.txt`:** Re-snapshot via `docs/runtime/scripts/souls_context_dumps_compiler.py`.

## Metricas de Sucesso
* `git grep "serverInfo.*souls\""` retorna 0 matches (substituido por `souls_mcp`).
* `git grep "targets\[0\].name: souls$"` retorna 0 matches em `gateway-config.yaml`.
* Suite TDD verde: `cargo test --bin souls_mcp_server` com `tools_list_respects_32_120_tetos` e os 2 snapshots existentes (`unprefixed`, `headroom_included`).
* Auditoria FinOps: soma de `len(server) + 1 + len(tool)` para todas as 40 tools <= 50 chars em media.

## Razao de Ser desta ADR
> "A Alma Matematica das tools (nomes curtos canibalizados) e o que importa. O brasao (servername) fica em uma unica torre: `souls_mcp`." — Bruno, 2026-08-02.
