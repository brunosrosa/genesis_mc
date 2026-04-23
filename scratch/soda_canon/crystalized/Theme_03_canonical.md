# Manual Canônico: Arquitetura SODA (Genesis MC)
**Curadoria:** Curador Arquitetural SODA
**Status:** Versão 1.0 - Consolidação de Infraestrutura de Agentes

---

## 1. Filosofia de Design: O "Porquê" Técnico
A arquitetura SODA rejeita a ineficiência do "brute-force reading" (leitura sequencial de arquivos inteiros por LLMs). O objetivo é a **Precisão de Contexto**. 

*   **Backend (Rust/Tokio):** O motor de execução. Garante concorrência de baixo nível e segurança de memória. A comunicação com o frontend é via IPC Zero-Copy, eliminando serialização redundante.
*   **Frontend (Svelte 5 + Tauri v2):** Interface puramente passiva. O estado é gerenciado no Rust; o Svelte atua apenas como uma camada de visualização reativa, sem lógica de negócio.
*   **Eficiência de Tokens:** A estratégia de "Munching" (indexação via Tree-sitter) transforma o repositório em um grafo de símbolos (funções, classes, métodos) com offsets de bytes, permitindo que o agente recupere apenas o código necessário, reduzindo o consumo de tokens em até 95%.

---

## 2. Auditoria Crítica: Fragilidades e Riscos
Após análise do material, identificamos pontos de falha na estratégia atual do Genesis MC:

1.  **Dependência de "uvx" e Ambientes Python:** A dependência de `uvx` para rodar MCP servers (como `jcodemunch` ou `duckduckgo-mcp`) introduz um gargalo de runtime. **Correção:** O SODA deve migrar para binários compilados em Rust que implementem o protocolo MCP nativamente, eliminando o overhead do interpretador Python.
2.  **Fragilidade de Web Scraping:** Servidores MCP baseados em navegadores (como o `google-ai-mode`) são suscetíveis a CAPTCHAs e bloqueios. **Correção:** O SODA deve priorizar APIs estruturadas ou scrapers baseados em `reqwest` (Rust) com headers de browser simulados, evitando a dependência de instâncias de navegador (Puppeteer/Playwright) que consomem memória excessiva da iGPU.
3.  **Conflito de "Vibe Coding":** A tendência de "vibecoding" (escrever código sem entender a estrutura) é um risco de segurança. O SODA deve impor **Hooks de Enforcement** (PreToolUse/PostToolUse) para validar se o agente está seguindo as diretrizes de arquitetura antes de aplicar qualquer alteração.

---

## 3. Diretivas de Hardware e Otimização
*   **Bare-Metal Execution:** O SODA deve rodar o indexador de código (Tree-sitter) utilizando diretivas AVX2 para acelerar o parsing de ASTs.
*   **RTX 2060m (Gargalo):** Para inferência local, o uso de `llama.cpp` com `mmap` é obrigatório. A alocação de VRAM deve ser estritamente monitorada para evitar o swap de barramento PCIe, que degrada a performance do sistema.
*   **Zero-Copy IPC:** Toda comunicação entre o backend Rust e o frontend Tauri deve utilizar `SharedMemory` ou `Bincode` sobre pipes, garantindo que o frontend não bloqueie o loop de eventos do backend.

---

## 4. Protocolo de Integração (MCP Canônico)
Para manter a integridade do SODA, qualquer novo servidor MCP deve aderir ao seguinte padrão:

1.  **Transporte:** Preferencialmente `stdio` para baixa latência. `SSE` apenas se houver necessidade de rede distribuída.
2.  **Formato de Resposta:** Deve suportar o formato "Compact Wire" (tags de caractere único para CSV/JSON) para minimizar o tráfego de tokens.
3.  **Auditoria:** Todo servidor deve expor a ferramenta `audit_agent_config` para verificar se o agente está operando dentro dos limites de tokens e escopo de pastas permitidos.

---

## 5. Eliminação de Ruído (Poda Tóxica)
*   **Proibido:** Qualquer menção a `Node.js` como daemon de backend, `Electron` para interface, ou `React` para o frontend. Estas tecnologias violam o princípio de "Arquitetura Pura" do SODA.
*   **Proibido:** SSR (Server-Side Rendering) via Next.js. O SODA é uma aplicação desktop nativa; o estado reside no cliente.

---

**Nota do Curador:** O Genesis MC deve focar na implementação de um **Indexador Nativo em Rust** que substitua a necessidade de servidores MCP baseados em Python. A precisão é a nossa única métrica de sucesso. **Mantenha o código limpo, o contexto preciso e o hardware frio.**