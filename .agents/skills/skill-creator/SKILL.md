---
## name: skill-creator 
description: Meta-habilidade estrutural ativada para destilar repositórios complexos e gerar novas habilidades agênticas. Ela orquestra a engenharia de Divulgação Progressiva, criando o SKILL.md e isolando lógicas em scripts atômicos. 
triggers: ["criar skill", "destilar repositório", "gerar habilidade", "nova skill", "criar agente", "skill-creator"]
---
## Goal

Atuar como a "Skill que cria Skills" (A Forja) dentro da infraestrutura do Genesis Mission Control (SODA). Sua missão é ler documentações densas ou ferramentas obscuras, extrair a "alma matemática" do processo e encapsulá-la no padrão arquitetural `agentskills.io` (Divulgação Progressiva em 3 Níveis). Você deve **proteger implacavelmente a VRAM (6GB)** do sistema, impedindo o _Context Rot_ ao isolar lógicas em scripts efêmeros e conhecimentos densos em arquivos de referência, mantendo o arquivo `SKILL.md` final enxuto e imperativo.

## Instructions

Sempre que o usuário solicitar a criação de uma nova Skill, execute a seguinte máquina de estados de forma sequencial e rigorosa:

1. **Fase de Destilação e Ingestão (Nível 0):**
    
    - Utilize ferramentas como `jcodemunch-mcp` ou `notebooklm-mcp` para ler a documentação, o repositório ou os arquivos fornecidos pelo usuário.
    - Não transcreva o repositório; **destile** o fluxo de trabalho exato e os contratos de interface necessários para operar a ferramenta.
        
2. **Fase de Scaffold Arquitetural (Criação de Pastas):**
    
    - Crie o diretório raiz da nova habilidade: `.agents/skills/<nome-da-nova-skill>/`.
    - Crie obrigatoriamente os subdiretórios: `/scripts/` e `/references/`.
        
3. **Fase de Isolamento em Black-Boxes (Nível 3 - Scripts):**
    
    - **Regra de Ouro:** O agente final NÃO DEVE pensar ou adivinhar lógicas de validação.
    - Extraia toda a lógica procedural complexa, regex, chamadas de API pesadas ou validações de linting e crie scripts executáveis na pasta `/scripts/` (ex: `.agents/skills/<nome-da-nova-skill>/scripts/validator.sh` ou `.bat`).
    - O arquivo `SKILL.md` apenas instruirá o agente a executar este script e ler o código de saída (Exit Code 0 para sucesso, 1 para falha).
        
4. **Fase de Descarregamento de Contexto (Nível 3 - References):**
    
    - Textos doutrinários densos, esquemas JSON imensos, contratos de API e regras de negócios profundas causam _Context Rot_ na janela primária do LLM.
    - Salve todo esse material estático em `.agents/skills/<nome-da-nova-skill>/references/rules.md`.
    - A nova skill apenas fará a amarração tardia (_Late-Binding_), mandando o agente ler este arquivo via comando cat/read apenas se necessário.
        
5. **Fase de Síntese do Frontmatter e SKILL.md (Nível 1 e 2):**
    
    - Escreva o arquivo principal `.agents/skills/<nome-da-nova-skill>/SKILL.md`.
    - **Nível 1:** O arquivo DEVE começar com um bloco YAML Frontmatter isolado por `---`, contendo `name`, `description` (escrita em terceira pessoa, focada no gatilho, max 100 tokens) e `triggers`.
    - **Nível 2:** O corpo Markdown DEVE conter as seções exatas: `## Goal`, `## Instructions`, `## Constraints`, e `## Examples`.
    - O arquivo `SKILL.md` final deve ter no máximo 150-200 linhas, atuando como um roteador que aponta para as pastas `/scripts/` e `/references/`.

## Constraints

- **Tolerância Zero para Node.js:** É ESTRITAMENTE PROIBIDO gerar scripts em `/scripts/` que dependam da inicialização de instâncias Node.js, ecossistemas NPM ou daemons Python em background. A arquitetura SODA é Bare-Metal.
- **Armas Nativas:** Force a dependência em binários nativos já presentes no sistema (Rust, Wasm, `curl`, `jq`, `grep`, `bash`, `powershell`) ou scripts atômicos puramente efêmeros.
- **Divulgação Progressiva:** Nunca injete o código-fonte de ferramentas de terceiros no corpo do `SKILL.md`.
- **Finitude do YAML:** O campo `description` do YAML frontmatter deve ser agressivamente enxuto. O agente "Mestre" lerá APENAS essa descrição para decidir se invoca o resto da pasta.

## Examples

**Usuário:** "Crie uma skill que faz auditoria de segurança em código Rust."

**Execução Correta do @skill-creator:**

1. Cria a pasta: `.agents/skills/rust-auditor/`
2. Cria o script: `.agents/skills/rust-auditor/scripts/validator.sh` (contendo as chamadas para `cargo audit` e `cargo clippy`).
3. Cria a referência: `.agents/skills/rust-auditor/references/owasp-rules.md` (contendo regras de segurança estáticas).
4. Cria o `.agents/skills/rust-auditor/SKILL.md` com o YAML contendo `name: rust-auditor` e instruções curtas mandando o agente executar `./scripts/validator.sh` e avaliar a saída contra o `/references/owasp-rules.md`.