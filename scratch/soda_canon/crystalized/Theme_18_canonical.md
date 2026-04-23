# Manual Canônico SODA - Eixo Temático 18: Sandbox de Agentes de IA

## 1. Introdução

Este manual consolida as informações sobre o "Sanity-Gravity", um sandbox para agentes de IA, com o objetivo de integrá-lo à arquitetura SODA. O Sanity-Gravity foca em isolar a execução de código arbitrário de agentes de IA em contêineres descartáveis, protegendo o sistema hospedeiro.

## 2. Arquitetura e Conceitos Fundamentais

O Sanity-Gravity opera com os seguintes princípios:

*   **Isolamento de Host:** A execução de agentes de IA ocorre dentro de contêineres Docker. Isso garante que ações destrutivas (como `rm -rf /`) ou a introdução de malware afetem apenas o ambiente isolado, preservando a integridade do sistema hospedeiro.
*   **Descartabilidade:** Os contêineres são projetados para serem descartáveis, minimizando a persistência de estados indesejados ou configurações corrompidas.
*   **Flexibilidade de Interface:** Suporta diferentes modos de interação:
    *   **Desktop GUI Completo:** Utiliza KasmVNC para fornecer uma experiência de desktop completa (Ubuntu 24.04 + XFCE4) acessível via navegador. Ideal para agentes que necessitam de interação visual ou uso de aplicações GUI.
    *   **Headless CLI:** Oferece imagens mínimas com acesso via SSH para agentes que operam exclusivamente em linha de comando (ex: Gemini CLI, Claude Code). Reduz a sobrecarga de recursos.
*   **Facilidade de Uso:** Projetado para ser "out-of-the-box", com ferramentas pré-instaladas como Antigravity IDE, Google Chrome e Git.

## 3. Componentes e Funcionalidades Chave

*   **`sanity-cli`:** A interface de linha de comando principal para gerenciar o ciclo de vida dos sandboxes. Permite iniciar (`up`), parar (`down`), pausar (`stop`/`start`), reiniciar (`restart`), limpar (`clean`), verificar status (`status`), construir imagens (`build`), listar tags (`list`) e interagir com o contêiner (`shell`).
*   **Sistema de Tags Modular:** As imagens Docker são construídas com um sistema de tags que define o agente, o desktop e o conector. Exemplos: `ag-xfce-kasm` (Antigravity IDE, XFCE, KasmVNC), `gc-none-ssh` (Gemini CLI, sem desktop, SSH).
*   **`gravity-cli`:** Um utilitário embarcado nos contêineres para gerenciar de forma segura a IDE (Antigravity) e o Google Chrome, utilizando `dpkg-divert` para proteger contra atualizações do sistema que poderiam quebrá-los.
*   **Proxy SSH Agent:** Permite que as chaves SSH do host sejam utilizadas dentro do contêiner sem a necessidade de copiá-las. Isso facilita operações como `git push`/`git pull` diretamente do sandbox.
*   **Multi-Instância:** Suporte para executar múltiplos sandboxes em paralelo, com alocação automática de portas para evitar conflitos.
*   **Snapshots de Container:** Capacidade de "congelar" um ambiente configurado (software instalado, sessões ativas) em uma nova imagem. Isso permite a criação de ambientes base reutilizáveis e a inicialização de novos sandboxes a partir desses snapshots.
*   **Persistência de Workspace:** Diretórios do host podem ser montados nos contêineres (`--workspace`), com mapeamento inteligente de UID/GID para evitar problemas de permissão de arquivos.

## 4. Integração com SODA e Considerações Arquiteturais

A arquitetura SODA, com Rust (Tokio) no backend e Svelte 5 + Tauri v2 no frontend, se beneficia do Sanity-Gravity ao:

*   **Isolamento de Execução de Agentes:** O Sanity-Gravity fornece o mecanismo de isolamento necessário para executar código de agentes de IA de forma segura. A lógica de orquestração e comunicação com esses agentes residirá no backend Rust.
*   **Frontend Passivo:** O frontend Svelte/Tauri pode ser utilizado para interagir com o `sanity-cli` (via chamadas IPC para o backend Rust) e para exibir o desktop remoto (KasmVNC) ou logs/saídas de CLI. O frontend não executará a lógica de sandbox.
*   **IPC Zero-Copy:** A comunicação entre o backend Rust e o frontend Tauri, bem como a orquestração do `sanity-cli`, deve ser otimizada para zero-copy, garantindo a máxima performance.

## 5. Auditoria Crítica e Pontos de Atenção

*   **Dependência de Docker:** O Sanity-Gravity depende intrinsecamente do Docker. A infraestrutura SODA deve garantir que o Docker esteja corretamente instalado e configurado no ambiente de execução.
*   **Gerenciamento de Recursos:** Embora o Sanity-Gravity permita definir limites de CPU e memória (`--cpus`, `--memory`), a gestão granular e a otimização desses recursos para cargas de trabalho de IA (especialmente aquelas que podem se beneficiar de otimizações de hardware) precisarão ser cuidadosamente orquestradas pelo backend Rust.
*   **Otimizações de Hardware (CPU/GPU):** O texto menciona otimizações como AVX2 para CPU e `mmap` para RTX 2060m (via `llama.cpp`). O Sanity-Gravity, por si só, não detalha como essas otimizações são expostas ou gerenciadas dentro dos contêineres. A arquitetura SODA precisará investigar e implementar a passagem dessas otimizações para os contêineres de sandbox, garantindo que o backend Rust possa direcionar a execução para o hardware apropriado. Gargalos de barramento da iGPU também devem ser considerados ao projetar a interação com modelos de IA que utilizam aceleração gráfica.
*   **Segurança do KasmVNC/VNC/SSH:** Embora o foco seja o isolamento do contêiner, a segurança das interfaces de acesso (KasmVNC, VNC, SSH) é crucial. Senhas fortes e a configuração adequada de redes são essenciais. O uso do proxy SSH Agent é um ponto positivo para evitar a exposição de chaves privadas.
*   **Atualizações e Manutenção:** O mecanismo de proteção contra atualizações (`dpkg-divert`) é uma boa prática, mas a estratégia de atualização do próprio Sanity-Gravity e das imagens base (Ubuntu) deve ser clara e integrada ao pipeline de CI/CD da SODA.
*   **Furo Potencial: Gerenciamento de Estado Persistente:** Embora os contêineres sejam descartáveis, o conceito de "workspace" e "snapshots" introduz um estado persistente. A estratégia de SODA para gerenciar e versionar esses workspaces e snapshots, especialmente em cenários de múltiplos usuários ou colaboração, precisa ser definida. Como garantir a integridade e a segurança desses dados persistentes?
*   **Furo Potencial: Complexidade de Configuração:** A modularidade do sistema de tags, embora poderosa, pode introduzir complexidade na seleção e configuração das imagens corretas para casos de uso específicos. O backend Rust da SODA precisará abstrair essa complexidade para o usuário final.

## 6. Conclusão

O Sanity-Gravity oferece uma base sólida para o isolamento e a execução segura de agentes de IA, alinhando-se com a necessidade da SODA de um backend robusto em Rust. A chave para uma integração bem-sucedida reside em orquestrar o `sanity-cli` a partir do backend Rust, gerenciar eficientemente os recursos de hardware (incluindo otimizações de CPU/GPU) e garantir que o frontend Svelte/Tauri atue como uma interface passiva e segura para a interação do usuário. A atenção aos pontos de auditoria crítica, especialmente em relação ao gerenciamento de estado persistente e otimizações de hardware, será fundamental para a robustez da arquitetura SODA.