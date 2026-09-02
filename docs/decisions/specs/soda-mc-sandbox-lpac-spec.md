# ESPECIFICAÇÃO TÉCNICA E CADERNO DE TDD: MARCO 5.13.0 (TASK 136)

## 🛡️ 1. Task 136 — Isolamento LPAC Nativo no Windows em `sandbox.rs`

### 1.1 Racional do Design (A Jaula de Silício do Windows 11)
A nossa ferramenta canônica `execute` (ou `souls_execute`) é a biela que executa compilações e suítes de testes geradas pela IA local de forma autônoma no Shadow Workspace para validar o ciclo TDD (o Ralph Loop). Atualmente, esta ferramenta opera sob um *stub* que retorna apenas simulações, pois executar códigos e testes arbitrários no Host principal sem isolamento físico representa um risco crítico de segurança.

A engenharia do SODA rejeita soluções de virtualização pesadas ou dependências do Docker no ambiente local do usuário, pois asfixiam a RAM Host e drenam os ciclos de clock que deveriam estar focados na inferência de modelagem de borda. 

A solução definitiva e bare-metal é transmutar o isolamento de processos para as primitivas nativas de segurança do **Windows 11**: o **Less Privileged AppContainer (LPAC)**, em conjunto com o NTFS Access Control Lists (ACLs) e **Windows Job Objects**.

### 1.2 O Less Privileged AppContainer (LPAC)
O LPAC é um perfil especializado do AppContainer do Windows introduzido para confinamento de processos com privilégios de I/O ainda mais severos do que o AppContainer padrão. Por padrão:
*   Um processo em AppContainer convencional tem acesso implícito a recursos de leitura do sistema e algumas subpastas de usuário.
*   Um processo em **LPAC** tem acesso de leitura/escrita **restrito a zero arquivos e diretórios**, a menos que o administrador ou o processo pai conceda acesso de forma explícita na ACL NTFS da pasta para o SID (Security Identifier) específico do container.
*   O LPAC vem com bloqueio total de rede por design (sockets locais, TCP/UDP e portas de loopback locais são inacessíveis, a menos que explicitamente autorizado).

---

## 💾 2. Engenharia e Arquitetura de Confinamento (`sandbox.rs`)

O módulo `src-tauri/src/core/sandbox.rs` implementará a criação da jaula física no Windows utilizando funções diretas da API Win32 fornecidas pela crate `windows-sys`:

### 2.1 Passos de Execução do Confinamento
1.  **Geração do Perfil:**  
    O Rust gera um nome de perfil pseudo-aleatório baseado no UUID do workspace e invoca `CreateAppContainerProfile` para registrar a identidade temporária no Windows Registry e obter o SID do AppContainer.
2.  **Preparação de Workspace no `%TEMP%`:**  
    O SODA estabelece o diretório do Shadow Workspace em `%TEMP%\.souls_workspaces\<id>\` (em disco NTFS, uma vez que o ProjFS não herda minifiltros no ReFS do Dev Drive Z:).
3.  **Mutação de NTFS ACLs (A Única Porta):**  
    O Rust invoca `SetNamedSecurityInfoW` para aplicar permissões explícitas e atômicas de leitura, escrita e execução (`GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE`) sobre o diretório do workspace especificamente para o SID do LPAC obtido. Qualquer outro caminho de arquivo no sistema operacional (incluindo o Host, o Dev Drive Z: e o registro) retorna instantaneamente `Access Denied` (HRESULT `0x80070005`) na cara do processo em sandbox.
4.  **Criação do Processo Enjaulado:**  
    O SODA inicializa o processo filho (ex: `cargo test` ou a compilação do teste) por meio do `CreateProcess` Win32, passando a struct `STARTUPINFOEXW` preenchida com a lista de atributos contendo o token de segurança do AppContainer obtido.
5.  **A Amarra de Job Objects (Morte Coletiva):**  
    Para mitigar picos de CPU infinitos e vazamentos de processos, o processo LPAC é anexado a um Windows Job Object configurado com limites rígidos de tempo de CPU e memória. Se o daemon principal do SODA sofrer shutdown, o Windows mata em cascata todos os processos filhos na jaula instantaneamente.

### 2.2 O Desvio de Segurança Gracioso (Bypass HRESULT 0x80070005)
Em conformidade com a nossa governança de não quebrar a suíte master de testes devido a restrições de permissões do sistema hospedeiro (como em ambientes sem elevação admin ou Session 0), o SODA implementará o **Bypass Gracioso**:
*   Se a chamada a `CreateAppContainerProfile` retornar o erro de ACL `0x80070005` (Access Denied), o sistema intercepta o erro de forma pacífica, desvia e aciona o nosso **Antivírus Bare-Metal O(1) de Retaguarda**:
    1.  O processo herda o isolamento simplificado via Windows Job Objects.
    2.  O SODA ativa a varredura estática de AST que lê o arquivo `Cargo.toml` e barra preventivamente qualquer repositório que execute arquivos `build.rs` arbitrários ou declare macros procedurais personalizadas que pudessem disparar execução de código remota (RCE) fora do workspace.

---

## 🚦 3. Caderno de Testes TDD (DoD GREEN)

Escreveremos e validaremos os seguintes testes físicos de segurança sob `cargo test --bin souls_mcp_server`:

1.  **`test_vram_scheduler_budget_calculation` (Preservado do Marco anterior):**  
    Continua assegurando a integridade e precisão matemática de VRAM e proteção de overflow.
2.  **`test_sandbox_lpac_creation`:**  
    Valida que o Rust consegue registrar um perfil LPAC temporário no Windows, derivar o SID e ler suas capacidades de herança sem emitir pânicos.
3.  **`test_sandbox_restricted_write`:**  
    Dispara um subprocesso dummy enjaulado no LPAC. O teste assevera que:
    *   O processo consegue escrever um arquivo texto temporário com sucesso dentro do seu workspace de teste autorizado.
    *   O processo falha catastoficamente com erro `Access Denied` ao tentar ler qualquer arquivo fora do workspace (como na pasta `Windows` ou no diretório principal `Z:\souls_mc\`).
4.  **`test_sandbox_network_isolation`:**  
    Instancia um teste de rede no processo enjaulado. Assevera que o LPAC barra imediatamente qualquer tentativa de abrir um socket TCP local (loopback `127.0.0.1:3001` do gataway) ou UDP, isolando a rede na origem.
