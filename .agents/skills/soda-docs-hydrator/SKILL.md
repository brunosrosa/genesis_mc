---
name: soda-docs-hydrator
description: O Guardião da Verdade Sintática do SODA. Vacina contra Vibe Coding, SEO Poisoning e Knowledge Cutoff. Impõe Hidratação Bilateral (Rust+Svelte), verificação estrita de SemVer (Svelte 5/Tauri v2) e extração atômica de Traits/Imports via MCP webcrawl antes da geração de código.
triggers: ["soda-docs-hydrator", "consultar documentação", "validar api", "buscar referências", "como usar a biblioteca", "docs", "verificar sintaxe"]
---

### skill: SODA Docs Hydrator (A Vacina Sintática V3.0)

#### Goal
Atuar como o Guardião da Verdade Sintática do SODA. Sua missão é combater a alucinação de código (Vibe Coding) gerada pelo viés de treinamento do LLM. Em um ecossistema de bibliotecas de vanguarda (Svelte 5 Runes, Tauri v2 IPC, iceoryx2, Tokio), a sua memória generativa não é confiável. Você deve hidratar seu contexto com a verdade oficial, blindando-se contra tutoriais legados e documentação fragmentada.

#### Instructions
Sempre que for solicitado a implementar uma funcionalidade usando os frameworks base do SODA, aplique a "Parada de Convicção" e obedeça a esta máquina de estados:

1. **A Parada de Convicção (Zero-Trust Interno):**
   * Assuma que a sua memória interna sobre Svelte e Tauri está defasada (focada em Svelte 4 e Tauri v1).
   * NÃO escreva código imediatamente.

2. **Pesquisa Anti-SEO e Filtro de Domínio:**
   * Invoque a ferramenta MCP de busca (`webcrawl_search` ou autorizada no Gateway).
   * Restrinja a busca aos domínios oficiais (ex: `site:svelte.dev/docs`, `site:v2.tauri.app`, `docs.rs`).
   * **Verificação SemVer:** Ao ler o conteúdo (`webcrawl_scrape`), verifique se o texto se refere expressamente à versão moderna (Svelte 5, Tauri v2). Se for Svelte 3/4 ou Tauri v1, descarte o texto imediatamente como "Envenenamento de SEO" e busque novamente.

3. **A Lei da Hidratação Bilateral (Para IPC e Full-Stack):**
   * Se a tarefa envolver a comunicação entre o Svelte 5 e o Rust, você está PROIBIDO de hidratar apenas um lado.
   * Você DEVE extrair a assinatura exata do *Frontend* (TypeScript/Svelte) E do *Backend* (Rust Macros) na documentação para garantir que o *Zero-Copy IPC* seja tipado corretamente.

4. **O Resgate do Código Órfão (Imports e Features):**
   * Ao memorizar a assinatura matemática da API, identifique e extraia OBRIGATORIAMENTE os *Traits* necessários (`use std::...`) e as *Feature Flags* exigidas para o arquivo `Cargo.toml` ou `package.json`. 

5. **Síntese O(1) e Descarte:**
   * Extraia apenas a assinatura, imports e o exemplo de uso minimalista.
   * **Descarte ativamente** o resto do lixo HTML da documentação da sua janela de contexto para proteger a VRAM local.
   * Projete o código no IDE respeitando estritamente a nova sintaxe.

#### Constraints
* **PROIBIÇÃO DE API LEGADA:** Qualquer submissão de código contendo padrões do Svelte 4 (como `export let`) ou Tauri v1 será sumariamente rejeitada pelo Ralph Loop.
* **PROIBIÇÃO DE ALUCINAÇÃO DE BINDINGS:** Não deduza as pontes IPC. Leia a documentação oficial.
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` é inegociável.

#### Examples
**Entrada do Usuário:** "Crie o listener de eventos de telemetria entre o Rust e o Svelte."

**Ação do Agente:**
1. Bloqueia o instinto de usar as APIs antigas do Tauri v1.
2. Busca a documentação do Tauri v2 filtrando por eventos IPC. Identifica a página correta e aplica a Hidratação Bilateral.
3. Lê a sintaxe TS para ouvir o evento (`listen`) e a sintaxe Rust para emitir (`app.emit()`).
4. Extrai a exigência da trait `use tauri::Emitter;` para o código compilar.
5. Descarta o resto do site da memória e gera o código perfeito no Antigravity IDE, avisando: *"Documentação do Tauri v2 (Rust + TS) e Svelte 5 Runes validadas. Traits importadas."*
