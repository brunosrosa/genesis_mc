---
name: weevolve
description: O Córtex de Aprendizado Relacional SODA. Extrai a "Alma Matemática" de bugs resolvidos e injeta as heurísticas no SQLite WAL. Prepara dados para compressão vetorial noturna (AutoDream), erradicando o Context Rot.
triggers: ["weevolve", "salvar heurística", "bug resolvido", "aprendizado contínuo", "extrair padrão", "documentar erro"]
---

# Skill: WeEvolve v2.0 (O Córtex de Aprendizado e Memória Tri-Partite)

## Goal
Erradicar a amnésia de sessão (Context Rot) e a dívida de VRAM causada pelo acúmulo de arquivos de texto de log contínuo. O objetivo é abstrair a resolução de um bug, erro de compilação ou falha de arquitetura em uma heurística matemática pura. Em vez de registrar o erro em arquivos Markdown, o agente deve estruturá-lo de forma relacional para ser ingerido pelo banco de dados do sistema (SQLite FTS5). Esta ação alimenta a Memória Episódica, que será processada de madrugada pelo *Chyros Daemon (AutoDream)* e convertida em tensores locais isolados.

## Instructions
Sempre que você solucionar um problema complexo ou receber a ordem de "aprender com isso", você DEVE executar estas ações em ordem estrita:

1. **Destilação da Alma Matemática:** Isole a falha das variáveis específicas do arquivo. Identifique o princípio arquitetural ou a regra física do sistema que foi violada.
2. **Formatação Heurística (Matriz 4D):** Você deve estruturar o aprendizado OBRIGATORIAMENTE em quatro eixos lógicos relacionais:
   - **The Insight (A Percepção):** O princípio subjacente descoberto.
   - **Why This Matters (A Relevância):** O sintoma exato que causará o colapso sistêmico se esta regra for ignorada futuramente.
   - **Recognition Pattern (Padrão de Reconhecimento):** Gatilhos semânticos, arquiteturas ou nomes de bibliotecas que indicam o risco de reincidência.
   - **The Approach (A Abordagem):** A regra inegociável de contorno.
3. **Injeção de Memória (Proibição de Textos Planos):** Você está expressamente PROIBIDO de realizar *append* ou salvar essas lições em arquivos de log contínuos como `.md` ou `.txt`. 
4. **Persistência Relacional:** Invoque a ferramenta MCP do banco de dados (SQLite) ou utilize o protocolo de escrita estruturada do ambiente para salvar esta matriz 4D como um registro isolado.
5. **Tag de Processamento Noturno:** Todo novo registro deve receber a tag interna `[AWAITING_AUTODREAM]`, sinalizando ao daemon noturno do SO que esta lição deve ser comprimida e vetorizada (Safetensors) durante a ociosidade do hardware.

## Constraints
* **PROIBIÇÃO DE LIXO SEMÂNTICO:** NÃO copie logs de erro de terminal ou *stacktraces* massivos. Salve estritamente a heurística deduzida.
* **PROIBIÇÃO DE MARKDOWN INFINITO:** Arquivos de log longos estouram os 6GB de VRAM do sistema alvo. Aja estruturando dados limpos.
* **MÁXIMA GENERALIZAÇÃO:** A regra deve prevenir a falha em qualquer arquivo futuro que use a mesma tecnologia, não apenas onde ocorreu o erro atual.

## Examples

**Entrada do Usuário:** 
"Finalmente passou no compilador. O erro era que o Tauri IPC precisa do `#[derive(serde::Serialize)]` para transitar dados Zero-Copy. Roda o weevolve pra IA nunca mais esquecer."

**Ação do Agente:**
1. O agente NÃO escreve em arquivos de texto.
2. Ele formata um *payload* estruturado (JSON/SQL):
   `Insight`: Tauri IPC exige serialização Serde para Zero-Copy.
   `Why`: Falha silenciosa no envio binário para a UI React.
   `Pattern`: Implementação de comandos `#[tauri::command]`.
   `Approach`: Injetar `#[derive(serde::Serialize)]` na struct de retorno.
3. O agente envia o payload para o banco SQLite do projeto com a tag `[AWAITING_AUTODREAM]`.