# MANUAL CANÔNICO: SODA (Sovereign Operating Data Architecture)
**Versão:** Genesis MC | **Status:** Arquitetura de Referência

---

## 1. O PORQUÊ TÉCNICO: A SOBERANIA DO BACKEND
A arquitetura SODA não é um framework de aplicação; é um **sistema de orquestração de dados soberanos**. A separação entre o *Frontend* (Svelte 5 + Tauri v2) e o *Backend* (Rust/Tokio) é absoluta.

*   **Frontend Passivo:** O Svelte 5 atua estritamente como uma camada de visualização. Toda lógica de negócio, validação de estado e processamento de dados é proibida no frontend.
*   **IPC Zero-Copy:** A comunicação entre o frontend e o backend deve utilizar o barramento IPC do Tauri v2 com serialização mínima. O objetivo é evitar a duplicação de memória e garantir que o backend mantenha a "fonte da verdade" (Single Source of Truth).
*   **Eliminação de Toxicidade:** Estão banidas quaisquer dependências de VDOM, Node.js daemons ou SSR (Next.js). A complexidade de um runtime de servidor é um vetor de ataque e um gargalo de performance inaceitável para o Genesis MC.

---

## 2. HARDWARE AWARENESS & OTIMIZAÇÃO BARE-METAL
O SODA opera sob restrições de hardware real. O desempenho não é negociável:

*   **Execução AVX2:** O backend deve ser compilado com diretivas de otimização de conjunto de instruções (target-cpu=native) para garantir que as operações de processamento de dados e inferência local utilizem as unidades vetoriais da CPU.
*   **Llama.cpp & mmap:** Para a RTX 2060m, a estratégia de carregamento de modelos deve utilizar `mmap` para mapear pesos diretamente na memória da GPU, minimizando a latência de transferência via barramento PCIe.
*   **Gargalos de iGPU:** O sistema deve monitorar o barramento da iGPU. Em cenários de alta carga, o SODA deve priorizar a descarga de tarefas de renderização para a GPU dedicada, evitando o *throttling* térmico e a contenção de memória compartilhada.

---

## 3. SEGURANÇA E SANDBOXING (LANDLOCK)
A soberania dos dados exige isolamento total. O SODA implementa o **Landlock LSM** para restringir o acesso a recursos do sistema:

*   **Política de Negação por Padrão:** Nenhum processo tem acesso ao sistema de arquivos ou rede, exceto o estritamente necessário.
*   **Sandboxing de Serviços:** Cada serviço do SODA é encapsulado via `landlock_restrict_self`. A configuração é declarativa (TOML), permitindo que o kernel Linux restrinja o acesso a caminhos específicos (`/tmp`, `/var/lib/soda`) sem a necessidade de namespaces complexos.
*   **Imutabilidade:** Uma vez que o sandbox é aplicado, ele não pode ser desativado. O SODA utiliza a versão 6.15+ do ABI do Landlock para garantir controle granular sobre sinais e sockets UNIX.

---

## 4. AUDITORIA CRÍTICA: FUROS E CONFLITOS
Após análise das fontes, identificamos pontos de fragilidade na estratégia do Genesis MC:

1.  **O Risco do CEL (Common Expression Language):** Embora o CEL seja eficiente, a fonte sugere que o acesso a `request.body` causa buffering em memória. **Furo:** Se o SODA utilizar CEL para validar payloads grandes, ele criará um gargalo de memória (OOM). **Correção:** O SODA deve implementar um *stream-parser* em Rust antes de passar qualquer metadado para o motor CEL.
2.  **Conflito de Composição de Políticas:** A fonte sobre Landlock menciona que políticas de segurança podem ser compostas, mas a "leveled down" (redução de privilégios) pode causar falhas silenciosas em serviços que esperam acesso total. **Aviso:** O SODA deve implementar um validador de políticas em tempo de compilação (Rust macros) para garantir que a composição de regras Landlock não resulte em um estado de "acesso negado" para operações críticas.
3.  **Fragilidade do `mmap`:** O uso de `mmap` para modelos na RTX 2060m é dependente da disponibilidade de memória contígua. Se o sistema operacional estiver sob pressão de memória, o `mmap` pode causar *page faults* massivos. **Mitigação:** O SODA deve implementar um *memory-pinning* para os pesos do modelo, garantindo que a memória não seja movida para o swap.

---

## 5. DIRETRIZ DE EXECUÇÃO
*   **Backend:** Rust (Tokio) para concorrência assíncrona.
*   **Frontend:** Svelte 5 (Reatividade fina).
*   **Segurança:** Landlock (FS/Network/Signal).
*   **Lógica de Acesso:** CEL (apenas para metadados, nunca para corpos de requisição brutos).

**Curador Arquitetural:** *A pureza do código é a única defesa contra a entropia do sistema.*