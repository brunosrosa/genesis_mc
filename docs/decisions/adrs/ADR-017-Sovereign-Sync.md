---
id: "ADR-017"
title: "ADR-017-Sovereign-Sync"
version: 1.0
status: Ativo_Inegociavel
epic: "Memória"
description: "Define o protocolo de sincronização eventual assíncrona entre o banco relacional SQLite e o Google Sheets."
---

# ADR-017-Sovereign-Sync

## Status
Aceito (Ativo e Inegociável)

## Contexto
Armazenar e sincronizar os históricos epistêmicos de raciocínio, notas pessoais da Tríade de Memória e decisões arquiteturais em servidores de nuvem centralizados de terceiros introduz riscos inaceitáveis de segurança, espionagem industrial e vazamento de privacidade. Mentes neurodivergentes e profissionais soberanos necessitam de garantias matemáticas absolutas de que seus dados cognitivos locais nunca sejam expostos. Soluções convencionais de sincronização baseadas em nuvem comercial impõem conexões pesadas, latências imprevisíveis e dependência de APIs fechadas proprietárias.

## Decisão
Fica estabelecido que toda sincronização de dados e consistência de estado entre máquinas licenciadas do usuário no ecossistema SOULS deve operar via **Sovereign Sync P2P Criptografado**:
1. **Rede Descentralizada (libp2p):** A comunicação e busca de peers na rede local ou rede aberta opera estritamente sem servidores intermediários centralizados, utilizando a biblioteca **libp2p** implementada nativamente no core Rust (Tokio).
2. **Versionamento Cabinet via Gitoxide:** O histórico imutável de snapshots de memória estruturada e eventos locais é versionado de forma assíncrona utilizando a engine **gitoxide** (100% Rust), eliminando dependências de CLIs de Git legadas lentas ou bibliotecas C inseguras (Libgit2).
3. **Criptografia Zero-Knowledge e Assinatura Digital:** Toda transmissão de dados trafega de ponta a ponta criptografada por chaves assimétricas **Ed25519** geradas localmente no host do usuário. Nenhum dado de memória semântica é transmitido ou salvo sem validação de assinatura e encriptação forte, blindando o conteúdo contra qualquer interceptação na rede.
4. **Governança B2B (OpenMLS + Silent Admin):** Para contas Enterprise, o Sovereign Sync incorpora o protocolo **OpenMLS**. Um **Silent Admin (Escrow)** é criado, permitindo que apenas o Gestor de TI local da empresa retenha a chave para decriptar a base dos funcionários, mantendo a nuvem do SOULS 100% cega.

## Consequências
- **Soberania Total:** Os dados pertencem exclusivamente ao usuário e estão fisicamente sob sua posse direta nos seus dispositivos Bare-Metal.
- **Resiliência Offline:** O SOULS mantém funcionalidade operacional completa offline, efetuando o sincronismo atômico assim que um peer confiável for descoberto na mesma rede local.
- **Robustez de Snapshot:** snapshots Gitoxide fornecem compactação de dados excepcional e capacidade de rollback seguro e reversão temporal da máquina de estados do cérebro.

## Restrições Bare-Metal
- **Latência de Inicialização libp2p:** A descoberta de peers e a conexão local via LAN devem estabelecer sessões em no máximo **1.5s** sob condições normais de rede.
- **Criptografia Assimétrica:** As rotinas criptográficas de codificação/decodificação na CPU i9 com vetores AVX2 devem executar em menos de **5ms** por bloco de snapshot transacionado.
- **Processamento Assíncrono:** As tarefas de versionamento do gitoxide e empacotamento P2P rodam isoladas em threads de background, consumindo zero recursos da thread principal de interação visual (Svelte 5).
- **I/O e Criptografia fora do Event Loop:** Operações do **gitoxide** (snapshots de backup, verificação de drift, rebases) e cálculos criptográficos pesados (**SHA-256**) são proibidos na thread principal do Tokio; devem ser descarregados para `tokio::task::spawn_blocking` ou *background workers* dedicados comunicando-se via **MPSC**, prevenindo starvation do Event Loop.
- **Enterprise OpenMLS:** Em contas Enterprise, o protocolo **OpenMLS** é obrigatório para governança B2B sem quebrar o Zero-Knowledge.
- **Silent Admin (Escrow Local):** A chave de decriptação corporativa deve ser retida exclusivamente pelo Gestor de TI local; a nuvem do SOULS permanece 100% cega ao conteúdo.
