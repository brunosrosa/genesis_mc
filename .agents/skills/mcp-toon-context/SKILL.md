---
name: mcp-toon-context
description: O Compressor de VRAM do SODA. Usa o servidor MCP Toon Context para minificar arquivos JSON densos, logs de erro massivos e retornos estruturados, removendo ruído sintático e protegendo a janela de contexto.
triggers: ["mcp-toon-context", "comprimir log", "minificar json", "reduzir tokens", "toon context", "limpar erro", "filtrar ruido"]
---

# Skill: MCP Toon Context (O Compressor de VRAM)

## Goal
Atuar como o escudo protetor da janela de contexto (VRAM) do SODA [1, 2]. O objetivo inegociável desta habilidade é impedir que você (o Agente) injete arquivos estruturados colossais (como JSONs brutos, logs de terminal ANSI ou árvores AST complexas) diretamente no seu contexto de raciocínio. Você deve invocar a ferramenta MCP `toon_compress` para destilar e minificar a "alma" semântica dos dados, descartando lixo de formatação, chaves redundantes e ruído sintático que causam a amnésia sistêmica (*Context Rot*) [3].

## Instructions
Sempre que você se deparar com a necessidade de ler um arquivo de configuração massivo, analisar um retorno de API denso ou quando a *skill* `@soda-ralph-loop` solicitar a leitura de um log de erro do compilador, você DEVE executar os passos abaixo:

1. **Interdição de Leitura Bruta:** Você está EXPRESSAMENTE PROIBIDO de ler arquivos JSON ou logs maiores que 2000 tokens diretamente usando comandos de leitura padrão (ex: `cat` ou `read_file`).
2. **Invocação do Compressor:** Encaminhe o caminho do arquivo alvo ou o texto bruto para a ferramenta `toon_compress` fornecida pelo servidor `toon-context`.
3. **Absorção Condensada:** Aguarde o retorno da ferramenta. O *output* será uma representação comprimida e limpa (formato TOON). 
4. **Análise Focada:** Utilize EXCLUSIVAMENTE o texto minificado para dar continuidade ao seu raciocínio, preservando a memória da GPU.

## Constraints
* **ZERO EXCEÇÕES PARA JSON/AST:** Arquivos de estrutura de dados (`.json`, `.yaml`, `.xml`) ou Árvores de Sintaxe Abstrata (AST) são os maiores vilões da VRAM. Eles devem, incondicionalmente, passar pelo filtro do Toon Context antes de serem interpretados.
* **MUTAÇÃO ZERO:** O Toon Context é um filtro de leitura. Ele não altera o arquivo original no disco do usuário. Confie no retorno dele como um "espelho limpo" do arquivo físico.

## Examples

**Entrada do Usuário:** 
"SODA, o script de build falhou. Dá uma olhada no log completo que gerou no `build_error.json` e me diz o problema."

**Ação Incorreta (NÃO FAÇA):**
O agente usa a ferramenta genérica de leitura de arquivos para ingerir 15.000 linhas de JSON formatado, estourando o contexto local e "esquecendo" as instruções de design do projeto.

**Ação Correta (Obrigatória):**
1. O agente aciona silenciosamente a ferramenta: `toon_compress(file_path="build_error.json")`.
2. A ferramenta remove os colchetes redundantes, timestamps inúteis e metadados de execução, devolvendo apenas as 12 linhas cruciais do erro de compilação.
3. O agente lê o retorno enxuto e aplica a correção no código correspondente, mantendo sua mente limpa e a VRAM estabilizada.
