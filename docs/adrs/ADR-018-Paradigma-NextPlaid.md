# ADR-018-Paradigma-NextPlaid

## Status
Aceito (Ativo e Inegociável)

## Contexto
O fatiamento (chunking) de código-fonte baseado em delimitadores cegos de caracteres (ex: blocos de 500 ou 1000 caracteres) ou divisores arbitrários de linhas quebra funções, desassocia documentações de suas implementações correspondentes, destrói assinaturas de tipos estruturais e corrompe o entendimento conceitual da IA. Na busca vetorial, chunks aleatórios misturam linguagens de programação e dependências causais, impossibilitando que agentes executem refatorações sistêmicas resilientes sem corromper a integridade sintática e sem sofrer de Context Rot.

## Decisão
Adotar rigidamente o **Paradigma NextPlaid** para o fatiamento e representação vetorial de códigos-fonte e arquivos de desenvolvimento no SODA:
1. **Fatiamento Orientado a AST:** É expressamente proibida a indexação de trechos brutos de caracteres fixos em arquivos de código. O chunking do código é executado cirurgicamente com base na sua **Árvore de Sintaxe Abstrata (AST)** utilizando a engine nativa `jcodemunch`.
2. **Ponto de Extração O(1):** A indexação decompõe o código-fonte em estruturas e símbolos semânticos explícitos (métodos, structs, enums, assinaturas de funções e blocos lógicos associados) mapeados por offsets geográficos exatos. Cada símbolo de código torna-se um vetor independente em L3 (LanceDB/LadybugDB) ancorado ao seu caminho lógico no repositório.
3. **Amarração Tarde de Relações:** O LadybugDB costura a teia causal conectando os nós de código fatiados por dependências de importação e chamadas de métodos, garantindo que o agente recupere a cadeia lógica inteira da assinatura ao buscar uma função correlata.

## Consequências
- **Precisão Cirúrgica de Código:** Agentes recuperam e operam sobre blocos funcionais completos e perfeitamente isolados sintaticamente, erradicando quebras do compilador por pedaços de código órfãos.
- **Eficiência de Contexto:** Redução maciça no tamanho dos payloads de RAG enviados ao contexto do LLM. O SODA injeta apenas o "coração da lógica" das funções correlacionadas, poupando a VRAM e minimizando custos operacionais.
- **Rigor de Navegação:** A navegação por AST fornece uma trilha de dependências inquebrável para auditorias dinâmicas e varreduras rápidas em $\mathcal{O}(1)$.

## Restrições Bare-Metal
- **Latência de Parsing AST:** A extração sintática O(1) de contornos de arquivos e assinaturas pelo motor `jcodemunch` deve rodar em no máximo **15ms** por arquivo normal em Rust.
- **Higiene Semântica:** Arquivos contendo lixo de desenvolvimento ou códigos temporários ignorados no Git principal estão sumariamente excluídos do fatiamento AST para prevenir contaminação da L3.
- **Consistência:** A atualização e re-indexação de nós de AST no LanceDB ocorre de forma incremental ativada em background assincronamente a cada salvamento bem-sucedido (Exit Code 0).
