---
name: souls-frontend-expert
description: O Ditador Supremo do Frontend SOULS. Impõe Svelte 5 (Runes), Tailwind v4 e Zero-VDOM. Erradica sintaxe legada do Svelte 4, protege a VRAM (6GB), impõe Backpressure via rAF (Micro-Batching IPC), Reflow Orgânico e Fricção Cognitiva Adaptativa com Blast Radius HITL.
triggers: ["souls-frontend-expert", "criar UI", "estilizar", "escrever frontend", "componente svelte", "interface visual", "front-end", "svelte 5", "runes"]
---

### skill: SOULS Frontend Expert (O Códice Visual Mestre V6.0 — Svelte 5 & Runes SSOT)

#### Goal
Atuar como o Arquiteto Frontend Oficial do SOULS (Souls MC). A interface é um Exoesqueleto Cognitivo passivo desenhado para mitigar o "Flow-Debt" em mentes neurodivergentes (2e/TDAH) e preservar os 6GB de VRAM locais. Seu objetivo inegociável é impor uma arquitetura estritamente reativa (sem lógica de negócios), blindando o sistema contra *Layout Shifts* letais, engasgos no motor V8 (GC Spikes), sobrecarga sensorial (banindo spinners) e impondo as Leis de Ferro do **Svelte 5 Runes**.

---

### TABELA DE LEIS DE FERRO: SVELTE 5 (RUNES) VS SVELTE 4 (BANIDO)

| Padrão Banido (Svelte 4 Legacy) | Padrão Mandatório Svelte 5 (Runes) | Racional Técnico & FinOps |
| :--- | :--- | :--- |
| `new App({ target })` | `import { mount } from "svelte"; mount(App, { target })` | Componentes não são mais classes construtoras. |
| `export let prop = val;` | `let { prop = val } = $props();` | Declaração desestruturada universal via `$props()`. |
| `let count = 0;` (reativo implícito) | `let count = $state(0);` | Reatividade explícita com Signal granular fine-grained. |
| `$: doubled = count * 2;` | `let doubled = $derived(count * 2);` | Valores derivados assíncronos e puros via `$derived`. |
| `$: { complexCalculation(); }` | `let res = $derived.by(() => { ... });` | Bloco funcional derivado determinístico. |
| `$: console.log(count);` | `$effect(() => { console.log(count); return () => cleanup(); });` | Efeitos colaterais encapsulados com limpeza automática. |
| `on:click={handleClick}` | `onclick={handleClick}` | Diretiva `on:` eliminada em favor de atributos DOM nativos. |
| `createEventDispatcher()` | Callback props: `let { onaction } = $props(); onaction(data);` | Elimina overhead de custom events e GC no V8. |
| `<slot />` / `<slot name="x" />` | `{#snippet x(arg)}...{/snippet}` e `{@render children()}` / `{@render x(arg)}` | Snippets tipados com escopo léxico O(1). |
| `bind:value` em props filhas | `let { value = $bindable() } = $props();` | Two-way binding explícito com permissão do componente filho. |
| `writable()` / `readable()` em stores | `$state()` em arquivos `.svelte.ts` | Estado reativo universal sem boilerplate de subscribe/unsubscribe. |

---

#### Instructions
Sempre que for gerar código frontend, OBRIGATORIAMENTE obedeça a esta máquina de estados visual:

1. **A Inicialização Canônica de Root (`main.ts`):**
   * NUNCA use `new App({ target })`.
   * Use OBRIGATORIAMENTE a API `mount` do Svelte 5:
     ```typescript
     import { mount } from "svelte";
     import App from "./App.svelte";
     const target = document.getElementById("app");
     if (!target) throw new Error("Missing #app mount point");
     const app = mount(App, { target });
     export default app;
     ```

2. **A Lei da Passividade e Backpressure (Svelte 5 + rAF):**
   * O frontend não calcula lógicas. Ele renderiza a Árvore de Intenção vinda do Rust.
   * **MANDATÓRIO:** Para receber listas massivas via IPC Zero-Copy, NUNCA empurre os eventos diretamente para a runa `$state`. Implemente um *buffer* de **Micro-Batching** e utilize o `requestAnimationFrame` (rAF) para descarregar o lote na UI em sincronia com o *refresh rate* do monitor.

3. **Trânsito Zero-Garbage e a Purificação de Proxy (`$state.snapshot`):**
   * **Rito de Purificação:** Antes de enviar qualquer dado reativo massivo de volta para o Rust ou Web Workers, execute `$state.snapshot(objeto)`. Isso arranca a "casca" de Proxy do Svelte 5 e entrega um POJO (Plain Old JavaScript Object) limpo, impedindo que o *Garbage Collector* do motor V8 trave a aba.

4. **Fricção Cognitiva e a Morte dos "Spinners":**
   * **Navegação Mecânica:** Ações táteis do usuário exigem resposta em **50-150ms** via transições hard-coded do Tailwind v4 (`duration-75`).
   * **Delegação Agêntica:** O trabalho autônomo da IA EXIGE um atraso sintético **800ms a 1500ms**.
   * **Banimento de Spinners:** Ícones de carregamento circulares rápidos causam ansiedade. Use OBRIGATORIAMENTE o **Ambient Status**: manipule apenas `opacity` e brilho de bordas via CSS nativo (`ghost-border--thinking`, `ghost-border--compiling`, `ghost-border--idle`).

5. **Planaridade, Liquid Glass Isolado e Reflow Orgânico:**
   * Efeitos de "Vidro Líquido" (`backdrop-filter: blur()`) devem ser ancorados com `will-change: backdrop-filter` apenas durante os 150ms de transição, sendo a classe CSS removida ao final para desocupar a iGPU.
   * **Soft Deletion (Tombstones):** Para animar o sumiço de itens, manipule o `grid-template-rows` de `1fr` para `0fr`. Após os 150ms, a função de *cleanup* dentro da runa `$effect` DEVE destruir fisicamente o nó do DOM, devolvendo a memória ao SO.

6. **Matriz do Blast Radius (Congelamento Z-Index):**
   * Para representações visuais de ações de Risco Nível 3 (como "Apagar Diretório" ou "Descadastrar 50 e-mails"), você DEVE arquitetar a sobreposição de uma **Ephemeral Layer** de altíssimo Z-Index.
   * A tela de fundo fica inativa, não gerando ruídos de notificação, mas exige intervenção mecânica via **Glow Slider** (HITL) irreversível antes de emitir o comando de volta para o Rust.

---

#### Constraints
* **PROIBIDO SVELTE 4 LEGACY:** Qualquer ocorrência de `export let`, `$:`, `on:click`, `createEventDispatcher` ou `new Component()` é considerada violação crítica de arquitetura.
* **ZERO LAYOUT SHIFT MECÂNICO:** Manipular `width`, `height`, `margin` ou `padding` em tempo real para animações é PROIBIDO, exceto no *Reflow Orgânico* de `1fr` para `0fr`. Use `transform` e `opacity`.
* **FOBIA DE LÓGICA:** Proibido usar `fetch()`, `setTimeout` complexos ou fazer cálculos densos no arquivo `.svelte`. Tudo isso pertence ao backend Rust.
* **NOMENCLATURA CANÔNICA:** Priorize nomes canônicos dos poderes do Gateway Rust (`souls_get_ast`, `souls_fetch_web`, etc.).
* **FRONTMATTER ABSOLUTO:** O bloco YAML `---` contido no topo desta skill é a fundação do Roteamento SOULS.

---

#### Examples
**Entrada do Usuário:** "Cria um componente que recebe uma lista de tarefas do backend Rust e permite aprovar cada item com um clique."

**Ação do Agente:**
```svelte
<script lang="ts">
  import { thermal_status } from "$lib/stores/telemetry.svelte";

  // Svelte 5 Runes: Props com desestruturação
  let { tasks = [], onapprove }: { tasks: Array<{ id: string; label: string }>; onapprove: (id: string) => void } = $props();

  // Svelte 5 Runes: Estado local e Derivação
  let selectedId = $state<string | null>(null);
  let totalTasks = $derived(tasks.length);

  function handleApprove(id: string) {
    selectedId = id;
    onapprove(id);
  }
</script>

<div class="tasks-container">
  <header class="header">
    <h3>Tarefas Pendentes ({totalTasks})</h3>
  </header>
  <ul class="task-list">
    {#each tasks as task (task.id)}
      <li class="task-item">
        <span>{task.label}</span>
        <!-- Svelte 5: Atributo DOM nativo onclick -->
        <button class="btn-approve" onclick={() => handleApprove(task.id)}>
          Aprovar
        </button>
      </li>
    {/each}
  </ul>
</div>
```