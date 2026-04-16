---
name: skill-creator
description: Meta-habilidade estrutural ativada para destilar repositórios complexos e gerar novas habilidades agênticas. Ela orquestra a engenharia de Divulgação Progressiva, criando o SKILL.md e isolando lógicas em scripts atômicos.
triggers: ["criar skill", "destilar repositório", "gerar habilidade", "nova skill", "criar agente", "skill-creator"]
---
# Meta-Skill: Skill Creator (A Forja de Habilidades SODA)

## Goal

Atuar como a "Skill que cria Skills" (A Forja) dentro da infraestrutura do Genesis Mission Control (SODA). Sua missão é ler documentações densas ou ferramentas obscuras, extrair a "alma matemática" do processo e encapsulá-la no padrão arquitetural `agentskills.io` (Divulgação Progressiva em 3 Níveis). Você deve proteger implacavelmente a VRAM (6GB) do sistema, impedindo o _Context Rot_ ao isolar lógicas em scripts efêmeros e conhecimentos densos em arquivos de referência, mantendo o arquivo `SKILL.md` final enxuto e imperativo.

## Instructions

### Passo 1: Levantamento de Dados (Ingestão do Conhecimento)

- Utilize ferramentas como `jcodemunch-mcp` ou `notebooklm-mcp` para ler a documentação, o repositório ou os arquivos fornecidos pelo usuário.
- Descarte sumariamente quaisquer lógicas que dependam de servidores web monolíticos pesados ou runtimes contínuos baseados em Node.js/Python em background, priorizando comandos de terminal nativos ou binários compilados (Rust/Wasm).

### Passo 2: Estruturação da Taxonomia de Diretórios

Prepare o terreno no sistema de arquivos usando estritamente o terminal local. Execute:

```bash
mkdir -p .agents/skills/<nome-da-nova-skill>/scripts/
mkdir -p .agents/skills/<nome-da-nova-skill>/assets/
mkdir -p .agents/skills/<nome-da-nova-skill>/references/
```

### Passo 3: Formatação e Escrita do SKILL.md

Você deve compor e escrever o arquivo SKILL.md utilizando a interface de terminal padrão (bash cat). O arquivo DEVE ter um YAML Frontmatter contendo name, description e triggers, seguido pelas seções Markdown ## Goal, ## Instructions, ## Constraints e ## Examples.

```bash
cat > .agents/skills/<nome-da-nova-skill>/SKILL.md << 'EOF'
[COLE O FRONTMATTER YAML E O CORPO MARKDOWN AQUI]
EOF
```

### Passo 4: A Forja do "Nível 3" (Caixa-Preta de Validação)

Para impedir que o LLM alucine validações, isole a verificação de erros da nova skill criando um script executável:

```bash
cat > .agents/skills/<nome-da-nova-skill>/scripts/validator.sh << 'EOF'
#!/usr/bin/env bash
# [SCRIPT DE VALIDAÇÃO GERADO AUTOMATICAMENTE]
# O LLM tratará o 'exit code' deste script como uma lei inegociável.
# Exemplo de falha: if [ erro ]; then exit 1; fi
echo "Validação concluída com sucesso."
exit 0
EOF
```

```bash
chmod +x .agents/skills/<nome-da-nova-skill>/scripts/validator.sh
```

### Passo 5: Ingestão de Referências (Evitando Context Rot)

Se a documentação tiver regras extensas (ex: padrões de arquitetura), NÃO as coloque no SKILL.md. Salve-as em references/rules.md:

```bash
cat > .agents/skills/<nome-da-nova-skill>/references/rules.md << 'EOF'
[TEXTO DOUTRINÁRIO DENSO]
EOF
```

### Constraints (Restrições Inegociáveis)

Economia de Tokens: A descrição no YAML deve ter no máximo ~100 tokens, pois residirá na memória constante. O nome da pasta da habilidade deve ser idêntico à propriedade name contida no YAML.
Zero Python/Node: Não crie arquivos .js ou .py para o agente rodar na nova skill. Restrinja-se a bash, Rust ou Wasm.
Examples
Entrada do Usuário: "Crie uma skill chamada soda-linter que roda cargo clippy." Ação do Agente: Executa mkdir -p .agents/skills/soda-linter/scripts, cria o SKILL.md com os triggers: ["validar rust", "rodar linter"] e cria um validator.sh contendo o comando cargo clippy -- -D warnings.