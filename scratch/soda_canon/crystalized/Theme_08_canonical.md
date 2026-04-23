# SODA: Manual Canônico de Arquitetura de Dados Soberanos (Gênese MC)

## 1. Introdução

O SODA (Sovereign Operating Data Architecture - Genesis MC) é um projeto que visa estabelecer uma arquitetura de dados robusta e soberana, priorizando o controle, a privacidade e a eficiência. Esta arquitetura é construída sobre pilares tecnológicos específicos para garantir a máxima performance e segurança.

## 2. Arquitetura Central

### 2.1. Backend: Rust (Tokio)

O backend do SODA é inteiramente desenvolvido em **Rust**, utilizando o framework assíncrono **Tokio**. Esta escolha é fundamental para:

*   **Performance Bare-Metal:** Rust oferece controle de baixo nível e ausência de garbage collector, permitindo otimizações próximas ao hardware e garantindo alta performance.
*   **Segurança de Memória:** O sistema de tipos e o modelo de propriedade do Rust previnem erros comuns de memória, como *data races* e *null pointer dereferences*, essenciais para a robustez.
*   **Concorrência Eficiente:** Tokio fornece um *runtime* assíncrono poderoso e eficiente, ideal para lidar com múltiplas requisições e operações I/O intensivas de forma escalável.
*   **IPC Zero-Copy:** A comunicação entre os componentes do backend e com o frontend é otimizada através de mecanismos de IPC (Inter-Process Communication) que minimizam ou eliminam a cópia de dados, crucial para a latência e o uso de recursos.

### 2.2. Frontend: Svelte 5 + Tauri v2

O frontend do SODA é construído com **Svelte 5** e empacotado com **Tauri v2**.

*   **Svelte 5:** Utiliza o novo modelo de execução do Svelte 5, que compila componentes em JavaScript imperativo e eficiente, evitando a sobrecarga de um Virtual DOM. Isso resulta em atualizações de UI rápidas e um *bundle* menor.
*   **Tauri v2:** Permite a criação de aplicações desktop nativas utilizando tecnologias web para a interface. Tauri é uma alternativa segura e performática ao Electron, utilizando o *runtime* nativo do sistema operacional e permitindo a comunicação eficiente com o backend Rust.
*   **Interface Passiva:** O frontend é estritamente uma interface **passiva**. Toda a lógica de negócio, processamento de dados e orquestração de LLMs reside no backend Rust. O frontend é responsável apenas pela apresentação visual e pela captura de interações do usuário, que são então enviadas ao backend via IPC.

## 3. Otimizações e Considerações de Hardware

A arquitetura SODA é projetada para ser *hardware-aware*, buscando extrair o máximo de performance do hardware disponível.

*   **CPU (AVX2):** Para inferência de LLMs que rodam primariamente na CPU, o código Rust deve ser compilado com otimizações para instruções **AVX2**. Isso permite paralelismo a nível de instrução, acelerando significativamente os cálculos.
*   **GPU (RTX 2060m / iGPU):**
    *   **llama.cpp mmap:** Para a RTX 2060m (ou GPUs com VRAM limitada), a estratégia de carregamento de modelos deve priorizar o uso de `mmap` (memory mapping) através de bibliotecas como `llama.cpp`. Isso permite que partes do modelo sejam carregadas sob demanda da memória do sistema (RAM) para a VRAM da GPU, contornando limitações de VRAM.
    *   **Gargalos de Barramento:** É crucial estar ciente das limitações de banda do barramento da iGPU (Integrated Graphics Processing Unit). Transferências de dados excessivas entre a CPU e a iGPU podem se tornar um gargalo significativo. O design do backend Rust deve minimizar essas transferências, preferencialmente mantendo os dados o mais próximo possível do processador que os utilizará.
*   **Gerenciamento de Memória:** Técnicas como **quantização** (redução da precisão dos pesos do modelo, ex: GGUF, GPTQ) e **offloading** (mover partes do modelo entre VRAM e RAM) são essenciais para rodar LLMs em hardware com VRAM limitada. A arquitetura deve suportar a configuração e o gerenciamento dessas técnicas.
*   **Model Sharding/Paralelismo:** Para modelos muito grandes, a arquitetura deve prever a capacidade de **sharding** (divisão do modelo em partes) e paralelismo (distribuição dessas partes entre múltiplos dispositivos, como múltiplas GPUs ou CPU+GPU), conforme suportado por bibliotecas subjacentes.

## 4. Lógica de Orquestração de LLMs

O SODA atua como um orquestrador inteligente para múltiplos LLMs, focando em otimização e controle.

*   **Proxy LLM:** A funcionalidade de proxy para LLMs é central. O backend Rust implementa um sistema que roteia requisições para diferentes modelos (locais ou via API) com base em critérios configuráveis.
    *   **Otimização de Custo/Resposta:** O sistema deve ser capaz de selecionar o LLM mais adequado para uma determinada tarefa, considerando fatores como custo de inferência, latência esperada e qualidade da resposta.
    *   **Gerenciamento de Chat History:** O backend deve gerenciar o histórico de conversas de forma eficiente, permitindo que ele seja passado corretamente para os LLMs, mantendo o contexto das interações. O formato do histórico (system, user, assistant) deve ser rigorosamente seguido.
*   **Ferramentas de Auto-Hospedagem:** O SODA se integra ou replica funcionalidades de ferramentas como Ollama, vLLM, LM Studio e AnythingLLM, mas sempre com a lógica central em Rust. O objetivo é fornecer uma experiência unificada e controlada, sem depender de interfaces ou runtimes externos para a lógica principal.
    *   **Exclusão de Tecnologias Indesejadas:** Qualquer menção ou sugestão de uso de **React, Node.js daemons, Electron, VDOM ou Server-Side Rendering (Next.js)** no contexto da lógica de aplicação ou da arquitetura central deve ser sumariamente **ignorada e removida**. O SODA é Rust-first, Svelte-last para a UI.

## 5. Auditoria Crítica e Pontos de Atenção

Ao analisar o material de referência, identificam-se potenciais "furos" ou fragilidades quando aplicados à visão SODA:

*   **Dependência de Ferramentas de Terceiros para Lógica:** Algumas ferramentas de auto-hospedagem (como Ollama, LM Studio) oferecem interfaces gráficas e APIs que podem ser tentadoras para serem usadas diretamente. **O SODA deve evitar essa dependência para a lógica central.** A lógica de orquestração, roteamento e otimização deve residir **exclusivamente no backend Rust**. As ferramentas externas podem ser usadas como *backends* de inferência, mas não como a camada de orquestração principal.
*   **Complexidade de Manutenção vs. Simplicidade de Uso:** Ferramentas como LM Studio e Ollama focam em simplificar a experiência do usuário final. Embora o SODA deva ser fácil de usar através de sua interface Svelte, a **complexidade inerente à otimização de LLMs (gerenciamento de memória, drivers, dependências)** não deve ser abstraída a ponto de esconder a necessidade de configuração e monitoramento no backend Rust. A simplicidade deve vir da arquitetura bem definida, não da ocultação de complexidade.
*   **Foco Excessivo em UI em Ferramentas de Referência:** Algumas ferramentas de auto-hospedagem dão grande ênfase à UI (Open WebUI, LM Studio GUI). No SODA, a UI é **passiva**. A inteligência e a complexidade residem no backend Rust. Qualquer arquitetura que proponha lógica de orquestração ou processamento no frontend é considerada **tóxica** e deve ser descartada.
*   **Privacidade e Soberania vs. APIs de Terceiros:** Embora o auto-hospedagem promova privacidade, a integração com APIs de LLMs proprietários (GPT-4, Claude) ainda pode envolver envio de dados para terceiros. O SODA deve priorizar LLMs que possam ser totalmente auto-hospedados para garantir a soberania completa dos dados. Se APIs externas forem usadas, isso deve ser uma configuração explícita e consciente dos riscos.
*   **Performance e Gargalos:** A menção a gargalos de barramento da iGPU e a necessidade de otimizações como `mmap` são críticas. O SODA deve ter mecanismos para monitorar e, se possível, mitigar esses gargalos através de estratégias de carregamento e processamento de dados eficientes no backend Rust. A simples delegação para bibliotecas genéricas sem controle fino pode não ser suficiente.

## 5. Conclusão

O SODA é uma arquitetura que une a performance e segurança do Rust/Tokio com a eficiência de UI do Svelte 5/Tauri v2. Seu objetivo é fornecer uma plataforma soberana para a orquestração e inferência de LLMs, com foco em otimizações de hardware, controle de dados e eficiência de custos, evitando ativamente arquiteturas e tecnologias consideradas "tóxicas" para este propósito.