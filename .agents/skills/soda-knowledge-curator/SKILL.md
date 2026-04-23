---
name: soda-knowledge-curator
description: A Mente Purificadora do SODA. Orquestra a deduplicação semântica via ferramentas matemáticas e a consolidação final de fontes via Doc Combiner. Erradica o Context Bloat no NotebookLM. Acionado para purgar bases retroativas, organizar arquivos fragmentados e limpar as referências da IA de conceitos antigos.
---

# Skill: SODA Knowledge Curator

## Goal
Atuar como o Guardião da Retenção Cognitiva e Higiene Semântica. O objetivo é impedir que fontes de pesquisa redundantes, desatualizadas e fragmentárias esgotem os recursos vetoriais de indexação e levem ao "Context Bloat".

## Instructions
1. Empregue extração atômica para puxar os documentos fontes crus (seja localmente ou extraindo massivamente do NotebookLM via `mcp_agent-gateway_notebooklm_source_get_content`).
2. Proponha ou instancie a execução de scripts Python em ambientes virtuais isolados (`scratch/.venv`) que utilizem lógicas de NLP matemáticas (`semhash`, FAISS).
3. Agrupe semanticamente os conteúdos, encontrando e expurgando redundâncias textuais estruturais independentemente do fraseado diferente, guiando-se por limiares restritos de similaridade de cosseno (ex: >0.95).
4. Aplique Resolução por Recência: perante conceitos que conflitam logicamente, identifique a versão defasada do conceito, execute a poda de contexto (*Context Pruning*) e valide apenas a anotação moderna.
5. Após o refino semântico absoluto, instrua ou execute o empacotamento das informações sobreviventes (ex: `notebooklm-doc-combiner`), unificando tudo e formatando cabeçalhos densos em `combined.md`.

## Constraints
- PROIBIÇÃO DE AGLOMERAÇÃO CEGA: Juntar milhares de páginas em formato bruto e não processado num único PDF/MD massivo apenas para driblar o limite quantitativo de fontes compromete a indexação do vetor e é terminantemente proibido.
- PROIBIÇÃO DE DELEÇÃO DESTRUTIVA SEM BACKUP: Jamais substitua e delete a matriz de fontes remota no NotebookLM ou no servidor MCP sem que haja um Shadow Workspace consolidando localmente e com confirmação estrita humana.
- IMPOSIÇÃO DE HIERARQUIA: Ao consolidar os Markdowns limpos finais, as tags (H1, H2, H3) devem estar semântica e logicamente ordenadas para a máquina interpretar quebra de contexto.

## Examples

Entrada do Usuário: "Ative o Modo de Purga Retroativa nas 249 fontes estilhaçadas na minha base atual."

Ação do Agente:
1. Extrai iterativamente o texto das 249 fontes, salvando os raw files no `scratch`.
2. Implementa e orquestra um script isolado `dedup.py` injetando bibliotecas matemáticas nativas.
3. O script funde as ramificações semânticas espúrias em apenas os recortes conceituais exclusivos.
4. Combina-os em `combined_master.md` preservando a estrita formatação do Markdown.
5. O agente reporta o delta quantitativo: "180 fontes eram ruído semântico deduzido, a superestrutura agora conta com densidade máxima pura."
