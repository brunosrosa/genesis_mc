---
name: mcp-jcodemunch-master
description: O manual de instrução inegociável para leitura cirúrgica de código. Proíbe a leitura de arquivos inteiros por força bruta. Força o uso de parsing via Árvore de Sintaxe Abstrata (AST) em O(1) para poupar VRAM.
triggers: ["mcp-jcodemunch-master", "ler código", "buscar função", "analisar classe", "ler repositório", "jcodemunch", "explorar código", "AST"]
---

# Skill: MCP JCodeMunch Master (Leitura Cirúrgica via AST)

## Goal
Proteger a janela de contexto do LLM e a VRAM da máquina (limite absoluto de 6GB) contra o *Context Rot* e o *Tool Bloat* induzidos pela leitura massiva de arquivos de código fonte [1, 2]. O objetivo é doutrinar o agente a abandonar ferramentas arcaicas (como `cat`, `read_file` geral ou `grep`) e utilizar exclusivamente a biblioteca `tree-sitter` através do servidor MCP `jcodemunch` [3]. A finalidade é extrair lógicas matemáticas e nós de código específicos em tempo $\mathcal{O}(1)$ e com economia de até 99% no custo de tokens [4].

## Instructions
Sempre que você precisar ler, entender ou auditar um arquivo de código-fonte ou repositório inteiro na "Fábrica" (Antigravity IDE), você DEVE operar sob a mecânica de Extração Cirúrgica via AST. 

Você deve executar as seguintes fases operacionais em ordem estrita:

1. **Interdição de Força Bruta:** Você está expressamente PROIBIDO de solicitar a leitura integral de grandes arquivos de código. Nunca injete documentos de código inteiros no seu contexto [1, 5].
2. **Mapeamento Estrutural (AST):** Utilize as ferramentas fornecidas pelo servidor MCP do `jcodemunch` para solicitar primeiramente apenas o **índice** ou a Árvore de Sintaxe Abstrata do arquivo ou diretório alvo [1]. Seu objetivo inicial é obter puramente os metadados estruturais (nomes de classes, assinaturas de funções, declarações de variáveis) [3].
3. **Extração O(1) por Byte-Offset:** Após analisar o mapa estrutural recebido, identifique o nó exato que contém a lógica que você precisa modificar ou entender. Faça uma segunda requisição ao MCP solicitando *exclusivamente* o conteúdo daquela função ou bloco lógico específico, utilizando o *byte-level offset* ou o identificador restrito do nó (AST node) [1, 6]. 

## Constraints
* **ECONOMIA DE TOKENS INEGOCIÁVEL:** A extração cirúrgica deve reduzir o consumo de leitura da sua memória em mais de 95%, caindo da faixa dos milhares de tokens para meras poucas centenas por tarefa [1, 3].
* **PURIFICAÇÃO DE RETORNO:** Ao receber o trecho de código extraído, aja ativamente como um filtro analítico: descarte mentalmente importações e comentários irrelevantes e foque estritamente na "alma matemática" da função antes de prosseguir com o *Spec-Driven Development* (SDD).
* **AMARRAÇÃO TARDIA (LATE-BINDING):** Lembre-se que você só carrega as assinaturas reais da ferramenta MCP no momento em que invoca a exploração. Libere o cache assim que a função exata for consertada.

## Examples

**Entrada do Usuário:** 
"Verifique como a função de serialização de payload na camada de IPC está lidando com o banco de dados no arquivo `ipc_handler.rs`."

**Ação Incorreta (NÃO FAÇA):**
O agente invoca ferramentas padrão de sistema para ler o `ipc_handler.rs` inteiro, inundando sua escassa VRAM com 15.000 tokens de código irrelevante, o que estrangula o raciocínio subsequente.

**Ação Correta (Obrigatória):**
1. O agente invoca a ferramenta MCP do `jcodemunch` (ex: `get_symbols` ou `index_file`) para listar apenas as assinaturas do arquivo `ipc_handler.rs`.
2. O servidor MCP responde com um esqueleto em JSON listando os *offsets* das funções, incluindo o método exato de `serialize_ipc_payload`.
3. O agente invoca o `jcodemunch` novamente pedindo a extração isolada do nó correspondente a `serialize_ipc_payload`.
4. O agente recebe apenas as 25 linhas estritas do método, processando a resposta com uso de VRAM insignificante e mantendo sua atenção perfeitamente clara para arquitetar a solução.