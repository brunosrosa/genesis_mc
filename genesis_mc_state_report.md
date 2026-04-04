# Genesis MC - Report de Estado do Workspace

**Data/Hora da Varredura:** 2026-03-21T17:50:32-03:00

## 1. Estrutura de Diretórios Principal

A estrutura atual conta com 9 subdiretórios e 14 arquivos na raiz:

**Diretórios Relevantes:**
- `.agents/` e `.beads/`: Sugerem integração com workflows baseados em IA e gestão de tarefas Beads ativa localmente.
- `.git/` e `.vscode/`: Controle de versão e configurações da IDE.
- `dist/`, `node_modules/`, `public/`, `src/`: Estrutura padrão de frontend Vite/React.
- `src-tauri/`: Diretório do backend em Rust/Tauri v2.

**Arquivos de Contexto e Documentação Encontrados:**
- `ARCHITECTURE.md`
- `DEVELOPMENT_PLAN.md`
- `PRODUCT_VISION.md`
- `PROMPT_MESTRE.md`
- `README.md`
- `UI_GUIDELINES.md`

Esta estrutura está altamente aderente ao contexto exigido para um projeto Tauri v2 bem documentado.

## 2. Regras Ativas (Global e Workspace)

### Regras Globais (Constituição do Agente):
1. **Identidade:** Postura sênior, direta, sem jargões como "delve", "fostering", etc. Uso de negrito para destaques vitais.
2. **Raciocínio Contextual:** Operação baseada no ciclo *Thought -> Action -> Observation -> Synthesis* obrigatoriamente.
3. **Orquestração Híbrida:** Protocolo ARC + RALPH Loop para testes iterativos (*Rule of Two* no máximo).
4. **TDD Backpressure:** Nenhuma submissão ou conclusão sem validação via testes/linters com "exit code 0".
5. **Higiene Windows:** Todo comando terminal assíncrono envelopado em `cmd /c` (ex: `cmd /c bd list`).
6. **Deeptutor:** Explicações diagnósticas estruturadas (Tabela de Contraste + 4 Passos).
7. **Segurança (Human Gates):** Bloqueio estrito para mutações perigosas, exigindo aprovação humana explícita.
8. **Git Flow:** Commits atômicos, mensagens semânticas, proibição de commit direto em `main` e sem código quebrado.
9. **Beads (Gestão de Tarefas):** Memória de estado gerida primariamente pela ferramenta `bd`. *(Nota: conexão com o DB Dolt na varredura inicial falhou. O servidor parece inativo).*

### Regras Locais (Workspace - Genesis MC):
1. **Stack Tecnológico Imutável:** Backend em Rust, Desktop UI em Tauri v2, Frontend em React 18+ (utilizado React 19.1.0 no package.json), Tailwind v4, Shadcn, Framer Motion, React Flow e Pragmatic Drag and Drop. Proibido Next.js, Electron ou bibliotecas CSS-in-JS.
2. **Arquitetura de Comunicação:** Frontend passivo ("burro"). Uso intensivo de Eventos Tauri IPC (`emit`/`listen`) em binário para dados grandes.
3. **Banco de Dados Dolt:** Exigência de *Graceful Sleep* na pool de conexões do Rust para aguardar o boot do Dolt.
4. **Design/Estética:** Componentes novos devem respeitar `UI_GUIDELINES.md` com micro-interações via Framer Motion ou Tailwind.
5. **Prevenção de Context Rot:** Separação rígida de contexto entre front e back. Leitura prévia de `INITIAL.md` para novas features.
6. **Controles de Segurança Físicos:** Alterações de FS ou Repositório que fujam do padrão exigem consentimento via UI (React).

## 3. MCPs, Extensões e Workflows Operantes

- **MCPs Conectados:**
  - **pencil:** Servidor especializado na leitura bidirecional e na geração de dados restritos para UI/Design focado em `.pen` files.
- **Workflows e Ferramentas:**
  - A pasta `.agents/` comprova suporte ativo a workflows customizados da arquitetura local.
  - A ferramenta **Beads** (`.beads/` core) é a extensão mestra para track de tarefas, mas **ATENÇÃO:** Encontra-se offline no momento (`Dolt server unreachable at 127.0.0.1:3307`).

## 4. Análise de Dependências e Configurações

**`package.json` (Frontend):**
- Utiliza **React 19.1.0** (o projeto exige 18+, está na versão mais atual, portanto aderente).
- Utiliza **Tailwind CSS v4.0.0** nativo via plugin do Vite (`@tailwindcss/vite`), conforme especificado.
- Presença forte das bibliotecas mandatárias do workspace: `@atlaskit/pragmatic-drag-and-drop`, `@xyflow/react`, `framer-motion`, pacotes relacionados ao `shadcn` e `zustand` para state management.
- *Observação:* O uso do pacote `tw-animate-css` é a única dependência adicional de estilização que foge da tríade pura do Tailwind/Framer/Shadcn, mas não viola restrições de "CSS-in-JS", logo, não sendo errada em natureza. 

**`src-tauri/Cargo.toml` (Backend Rust/Tauri v2):**
- Configuração limpa e perfeitamente padronizada para o **Tauri v2** (`tauri` e `tauri-plugin-opener` v2), dependendo de `serde` e `serde_json`. Não há nenhuma biblioteca órfã ou import experimental/perigoso visível aqui no momento.

**Conclusão sobre Dependências:**
Não foram identificadas dependências órfãs preocupantes ou configurações abertamente fora do padrão arquitetural estipulado. O workspace encontra-se tecnicamente são, com a única pendência infraestrutural sendo a inicialização do container Dolt para o sistema Beads.
