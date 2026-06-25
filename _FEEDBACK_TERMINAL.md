================================================================
 🦅 SODA GENESIS MC - PAINEL DE IGNIÇÃO ETL V5 (JANELA DE VIDRO)
================================================================

[+] Calibrando Reator: Injetando chaves do .env na memória...
[OK] Chaves de API e Google Sheets injetadas com sucesso.
Modo de execução: [ENTER] Normal  [2] Dry-run (1 rodada):

MODO: EXECUÇÃO NORMAL

SELECIONE A ENGRENAGEM DE EXECUÇÃO:
----------------------------------------------------------------
 [0] 👁️  N0 - Daemon Watcher (Cron Job)
             (Acorda o Olheiro Assíncrono para verificar novos links)
 [1] 🛡️  N1 - Guardião (Fase -1)
             (Prioriza NOVO_LINK_OK; depois roda o batch amplo) (Custo Zero)
 [2] 🛰️  N2 - Batedor FinOps (Fase -0.5) (IA Flash)
             (Busca README truncado + JSON Mode barato + Triagem Estruturada)
 [3] 🚜  N3 - Harvester Local (Fase 0)
             (Extração local O(1) para o SQLite Vault do RAW (Blobs)) (Custo Zero) (gatilho: APROVADO_PARA_HARVESTER)
 [4] 🧠  N4 - Motor Cloud Cognitivo (Fases 1, 2, 3 e 4) (IA Heavy)
             (Destilador + Enxame + Sintetizador + Injeção no GSheets) (gatilho: APROVADO_PARA_ENXAME)
 [5] 🤹🏻‍♀️  N5 - Revisão ETL Cognitivo Pesado (Fases 3 e 4) (IA Heavy)
             (Sintetizador + Escrita (Injeção) no GSheets) (gatilho: APROVADO_PARA_ENXAME)
 [6] 🔬  N6 - Deep Components Formatter (Fase 5)
             (Escreve a aba DEEP_COMPONENTS) (gatilho: APROVADO_DEEP_COMPONENTS_ANALYSIS)
 [X] 🛑  Abortar Ignição
----------------------------------------------------------------

Arquiteto, informe a rota de voo: 3
Informe o nome do Lote (Ex: LOTE_01_UX) ou deixe em branco para o padrao:

================================================================

🚀 DISPARANDO O MOTOR EM RUST (TOKIO EVENT LOOP)...

   Compiling tree-sitter-language-pack v1.9.1
   Compiling genesis_mc v0.1.0 (Z:\genesis_mc\src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 22s
     Running `target\debug\f0_harvester_cli.exe --batch`
[PROC] SODA F0 (Harvester/Zero-IA): modo batch sequencial gate=APROVADO_PARA_HARVESTER
[PROC] F0(batch): fila carregada count=5
[PROC] F0(batch): iniciando repo_id=mendableai/firecrawl row_number=306 idx=1 total=5
[PROC] Iniciando HarvesterOrchestrator (N14) url=https://github.com/mendableai/firecrawl repo_id=mendableai/firecrawl
[PROC] N1: Alocando workspace efemero da F0 repo_id=mendableai/firecrawl requested_mb=256
[PROC] N1: Workspace efemero pronto repo_id=mendableai/firecrawl workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100
[PROC] N2: Iniciando clone blobless repo_id=mendableai/firecrawl url=https://github.com/mendableai/firecrawl
[PROC] Preparando workspace efemero do clone url=https://github.com/mendableai/firecrawl workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100 dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl
[PROC] ProjFS: consultando metadados do repositório GitHub url=https://api.github.com/repos/mendableai/firecrawl
[PROC] ProjFS: consultando release mais recente do repositório url=https://api.github.com/repos/mendableai/firecrawl/releases/latest
[PROC] ProjFS: consultando SHA do commit HEAD url=https://api.github.com/repos/mendableai/firecrawl/commits?sha=main&per_page=1
[PROC] ProjFS: baixando snapshot compactado do repositório url=https://api.github.com/repos/mendableai/firecrawl/zipball/main default_branch=main selected_branch=main
[PROC] ProjFS: snapshot ZIP recebido em memória archive_bytes=30218528
[FINOPS] Clone virtual via ProjFS concluido url=https://github.com/mendableai/firecrawl dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl projected_files=1557 projected_bytes=49982706 elapsed_ms=13829
[OK] N2: Clone blobless concluido repo_id=mendableai/firecrawl repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl
[PROC] N3: Criando sandbox efemero repo_id=mendableai/firecrawl repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl
[PROC] N3: Sandbox pronto repo_id=mendableai/firecrawl
[PROC] N4: Detectando stack do repositório repo_id=mendableai/firecrawl
[PROC] N4: Stack detectada repo_id=mendableai/firecrawl profile=Mixed([Rust, Elixir, NodeJS, Go, Python, JVM, DotNet])
[PROC] N5: Roteando tarefas de extração repo_id=mendableai/firecrawl
[PROC] N5: Tarefas roteadas repo_id=mendableai/firecrawl tasks=[RunNativeAstParser, DiscoverTests, ExtractManifests, RunStaticAnalysis, FetchCommunityMeta, ExtractOpsBlueprint, RunOxc]
[PROC] N10: Iniciando coleta concorrente de metadados comunitarios repo_id=mendableai/firecrawl
[PROC] ast-native: iniciando extração estrutural repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=30
[PROC] ast-native: artefatos normalizados repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl repo_outline_bytes=449350 architecture_map_bytes=46349 health_report_bytes=290
[FINOPS] N6: parser AST nativo concluido repo_id=mendableai/firecrawl elapsed_ms=28241 repo_outline_bytes=449350 architecture_map_bytes=46349
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_04_repo_outline payload_bytes=449350
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_05_architecture_map payload_bytes=46349
[PROC] N7: Extraindo blob_01_promessa_readme repo_id=mendableai/firecrawl
[PROC] Tentando ler arquivo para artefato artifact_type=blob_01_promessa_readme candidate=README.md abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\README.md
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_01_promessa_readme payload_bytes=19164
[PROC] N8: Extraindo blob_02_dependency_manifest repo_id=mendableai/firecrawl
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=.github/scripts/requirements.txt abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\.github\scripts\requirements.txt
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/api/native/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/api/native/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/api/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/api/sharedLibs/go-html-to-md/go.mod abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\sharedLibs\go-html-to-md\go.mod
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/dot-net-sdk/Firecrawl/Firecrawl.csproj abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\dot-net-sdk\Firecrawl\Firecrawl.csproj
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/dot-net-sdk/Firecrawl.Tests/Firecrawl.Tests.csproj abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\dot-net-sdk\Firecrawl.Tests\Firecrawl.Tests.csproj
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/elixir-sdk/mix.exs abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\elixir-sdk\mix.exs
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/go-html-to-md-service/go.mod abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\go-html-to-md-service\go.mod
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/go-sdk/go.mod abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\go-sdk\go.mod
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/java-sdk/build.gradle.kts abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\java-sdk\build.gradle.kts
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/js-sdk/firecrawl/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\js-sdk\firecrawl\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/js-sdk/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\js-sdk\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/php-sdk/composer.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\php-sdk\composer.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/playwright-service-ts/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\playwright-service-ts\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/python-sdk/pyproject.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\python-sdk\pyproject.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/python-sdk/requirements.txt abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\python-sdk\requirements.txt
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/ruby-sdk/Gemfile abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\ruby-sdk\Gemfile
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/rust-sdk/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\rust-sdk\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/test-site/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-site\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/test-suite/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-suite\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=apps/ui/ingestion-ui/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\ui\ingestion-ui\package.json
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_02_dependency_manifest payload_bytes=4850
[PROC] N9: Extraindo blob_07_ops_blueprint repo_id=mendableai/firecrawl
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_07_ops_blueprint payload_bytes=107570
[PROC] N11: Extraindo blob_03_test_intent repo_id=mendableai/firecrawl
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_03_test_intent payload_bytes=145957
[PROC] N11: Extraindo blob_11_ux_contracts repo_id=mendableai/firecrawl
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_11_ux_contracts payload_bytes=2969
[PROC] SAST monorepo: manifestos detectados repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl manifest_count=15 manifests=["apps/api:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\api\\package.json", "apps/api/native:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\api\\native\\Cargo.toml", "apps/api/native:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\api\\native\\package.json", "apps/api/sharedLibs/go-html-to-md:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\api\\sharedLibs\\go-html-to-md\\go.mod", "apps/elixir-sdk:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\elixir-sdk\\mix.exs", "apps/go-html-to-md-service:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\go-html-to-md-service\\go.mod", "apps/go-sdk:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\go-sdk\\go.mod", "apps/js-sdk:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\js-sdk\\package.json", "apps/js-sdk/firecrawl:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\js-sdk\\firecrawl\\package.json", "apps/playwright-service-ts:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\playwright-service-ts\\package.json", "apps/rust-sdk:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\rust-sdk\\Cargo.toml", "apps/test-site:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\test-site\\package.json", "apps/test-suite:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\test-suite\\package.json", "apps/ui/ingestion-ui:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\apps\\ui\\ingestion-ui\\package.json", "examples/scrape_and_analyze_airbnb_data_e2b:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782280236050848100\\repos\\mendableai\\firecrawl\\examples\\scrape_and_analyze_airbnb_data_e2b\\package.json"] concurrency_limit=3
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=apps/rust-sdk cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\rust-sdk concurrency_limit=3 in_flight=2
[PROC] SAST monorepo: permissão adquirida blade=sobelow scope=apps/elixir-sdk cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\elixir-sdk concurrency_limit=3 in_flight=3
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=apps/api/native cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native concurrency_limit=3 in_flight=1
[OK] Sandbox: processo efemero concluido command=mix pid=11120 exit_code=1 stdout_bytes=0 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\elixir-sdk
[PROC] SAST monorepo: sub-scan concluído blade=sobelow scope=apps/elixir-sdk cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\elixir-sdk available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/native/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=2372 exit_code=0 stdout_bytes=231 stderr_bytes=103 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\src
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/native/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\scripts concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=20920 exit_code=1 stdout_bytes=453 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\scripts
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\scripts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/controllers/v0 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v0 concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=35168 exit_code=1 stdout_bytes=33582 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v0
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/controllers/v0 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v0 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/controllers/v1 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v1 concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=29608 exit_code=1 stdout_bytes=74905 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v1
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/controllers/v1 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v1 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/controllers::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=32516 exit_code=1 stdout_bytes=5197 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/controllers::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/db cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\db concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=23868 exit_code=1 stdout_bytes=4766 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\db
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/db cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\db available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib/branding cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\branding concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=32124 exit_code=1 stdout_bytes=21803 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\branding
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib/branding cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\branding available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib/deep-research cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deep-research concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=36860 exit_code=1 stdout_bytes=7303 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deep-research
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib/deep-research cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deep-research available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/controllers/v2 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v2 concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=31852 exit_code=1 stdout_bytes=123293 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v2
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/controllers/v2 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v2 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib/deterministicJson cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deterministicJson concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=27292 exit_code=1 stdout_bytes=8229 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deterministicJson
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib/deterministicJson cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deterministicJson available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib/extract cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\extract concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=60
[OK] Sandbox: processo efemero concluido command=biome pid=13952 exit_code=1 stdout_bytes=112043 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\extract
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib/extract cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\extract available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib/generate-llmstxt cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\generate-llmstxt concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=12980 exit_code=1 stdout_bytes=4590 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\generate-llmstxt
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib/generate-llmstxt cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\generate-llmstxt available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib/scrape-interact cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\scrape-interact concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=24208 exit_code=1 stdout_bytes=7872 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\scrape-interact
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib/scrape-interact cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\scrape-interact available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=23816 exit_code=1 stdout_bytes=35149 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib::files-02 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=39464 exit_code=1 stdout_bytes=25961 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib::files-02 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib::files-03 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=18700 exit_code=1 stdout_bytes=24401 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib::files-03 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/lib::files-04 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=37652 exit_code=1 stdout_bytes=1324 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/lib::files-04 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=90
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/main cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\main concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=37388 exit_code=1 stdout_bytes=2728 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\main
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/main cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\main available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/routes cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\routes concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=2476 exit_code=1 stdout_bytes=11230 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\routes
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/routes cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\routes available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/scraper/WebScraper cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\WebScraper concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=24132 exit_code=1 stdout_bytes=22165 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\WebScraper
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/scraper/WebScraper cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\WebScraper available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/scraper/crawler cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\crawler concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=22792 exit_code=1 stdout_bytes=1970 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\crawler
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/scraper/crawler cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\crawler available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/scraper/scrapeURL cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\scrapeURL concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=18476 exit_code=1 stdout_bytes=187842 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\scrapeURL
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/scraper/scrapeURL cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\scrapeURL available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/search cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\search concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=15596 exit_code=1 stdout_bytes=15766 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\search
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/search cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\search available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/services/alerts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\alerts concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=38392 exit_code=1 stdout_bytes=438 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\alerts
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/services/alerts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\alerts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/services/autumn cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\autumn concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=19968 exit_code=1 stdout_bytes=24512 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\autumn
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/services/autumn cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\autumn available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src/services::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=120
[OK] Sandbox: processo efemero concluido command=biome pid=10924 exit_code=1 stdout_bytes=30616 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src/services::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/api/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=17112 exit_code=1 stdout_bytes=12637 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/api/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/js-sdk/firecrawl/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\js-sdk\firecrawl\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=3000 exit_code=1 stdout_bytes=140865 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\js-sdk\firecrawl\src
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/js-sdk/firecrawl/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\js-sdk\firecrawl\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/playwright-service-ts cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\playwright-service-ts concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=10308 exit_code=1 stdout_bytes=4306 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\playwright-service-ts
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/playwright-service-ts cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\playwright-service-ts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/test-suite cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-suite concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=39988 exit_code=1 stdout_bytes=1551 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-suite
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/test-suite cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-suite available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/ui/ingestion-ui/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\ui\ingestion-ui\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=2768 exit_code=1 stdout_bytes=8905 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\ui\ingestion-ui\src
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/ui/ingestion-ui/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\ui\ingestion-ui\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=examples/scrape_and_analyze_airbnb_data_e2b cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\examples\scrape_and_analyze_airbnb_data_e2b concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=35908 exit_code=1 stdout_bytes=5771 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\examples\scrape_and_analyze_airbnb_data_e2b
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=examples/scrape_and_analyze_airbnb_data_e2b cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\examples\scrape_and_analyze_airbnb_data_e2b available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=apps/test-site/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-site\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=28944 exit_code=1 stdout_bytes=20222 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-site\src
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=apps/test-site/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-site\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/native/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=240 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\src
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/native/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\scripts concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=38804 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\scripts
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\scripts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/controllers/v0 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v0 concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=21708 exit_code=0 stdout_bytes=5173 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v0
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/controllers/v0 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v0 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/controllers/v1 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v1 concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=41648 exit_code=0 stdout_bytes=108609 stderr_bytes=3840 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\rust-sdk
[OK] Sandbox: processo efemero concluido command=oxlint pid=27892 exit_code=0 stdout_bytes=13440 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v1
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/controllers/v1 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v1 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/controllers/v2 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v2 concurrency_limit=3 in_flight=3
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\rust-sdk
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=apps/rust-sdk cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\rust-sdk available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/controllers::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=44716 exit_code=101 stdout_bytes=284424 stderr_bytes=9862 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native
[ERR] Sidecar terminou com exit code nao zero binary=cargo exit_code=101 stderr=Updating git repository `https://github.com/firecrawl/calamine`
    Updating crates.io index
    Updating git repository `https://github.com/firecrawl/nodesig`
    Blocking waiting for file lock on package cache
     Locking 320 packages to latest compatible versions
      Adding cfb v0.10.0 (available: v0.14.0)
      Adding generic-array v0.14.7 (available: v0.14.9)
      Adding lol_html v2.9.0 (available: v3.0.0)
      Adding roxmltree v0.20.0 (available: v0.21.1)
      Adding zip v5.1.1 (available: v8.6.0)
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
 Downloading crates ...
  Downloaded psl v2.1.214
  Downloaded pdf-inspector v0.1.3
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.46
   Compiling unicode-ident v1.0.24
   Compiling cfg-if v1.0.4
   Compiling zerocopy v0.8.52
   Compiling version_check v0.9.5
    Checking memchr v2.8.2
    Checking stable_deref_trait v1.2.1
    Checking typenum v1.20.1
    Checking windows-link v0.2.1
   Compiling siphasher v1.0.3
   Compiling siphasher v0.3.11
    Checking smallvec v1.15.2
   Compiling getrandom v0.3.4
   Compiling getrandom v0.1.16
    Checking log v0.4.33
   Compiling serde_core v1.0.228
   Compiling getrandom v0.2.17
   Compiling syn v1.0.109
   Compiling serde v1.0.228
   Compiling autocfg v1.5.1
   Compiling rand_core v0.6.4
    Checking subtle v2.6.1
   Compiling phf_shared v0.8.0
   Compiling generic-array v0.14.7
   Compiling phf_shared v0.13.1
    Checking new_debug_unreachable v1.0.6
   Compiling crc32fast v1.5.0
   Compiling fastrand v2.4.1
   Compiling phf_shared v0.11.3
   Compiling phf_shared v0.10.0
   Compiling find-msvc-tools v0.1.9
    Checking writeable v0.6.3
   Compiling crossbeam-utils v0.8.21
   Compiling shlex v2.0.1
   Compiling rand_core v0.5.1
    Checking simd-adler32 v0.3.9
   Compiling phf_generator v0.13.1
    Checking foldhash v0.2.0
   Compiling proc-macro-hack v0.5.20+deprecated
    Checking equivalent v1.0.2
   Compiling parking_lot_core v0.9.12
   Compiling jobserver v0.1.34
    Checking allocator-api2 v0.2.21
   Compiling rand_pcg v0.2.1
    Checking litemap v0.8.2
    Checking itoa v1.0.18
    Checking dtoa v1.0.11
    Checking cpufeatures v0.2.17
    Checking precomputed-hash v0.1.1
   Compiling cc v1.2.65
    Checking scopeguard v1.2.0
   Compiling pkg-config v0.3.33
    Checking dtoa-short v0.3.5
    Checking adler2 v2.0.1
   Compiling icu_normalizer_data v2.2.0
    Checking utf8_iter v1.0.4
    Checking crypto-common v0.1.7
   Compiling syn v2.0.118
    Checking block-buffer v0.10.4
    Checking block-padding v0.3.3
   Compiling icu_properties_data v2.2.0
    Checking miniz_oxide v0.8.9
    Checking lock_api v0.4.14
    Checking inout v0.1.4
    Checking digest v0.10.7
    Checking hashbrown v0.17.1
    Checking aho-corasick v1.1.4
    Checking encoding_rs v0.8.35
    Checking byteorder v1.5.0
    Checking regex-syntax v0.8.11
    Checking mac v0.1.1
    Checking cipher v0.4.4
    Checking pin-project-lite v0.2.17
    Checking zlib-rs v0.6.4
    Checking sha2 v0.10.9
    Checking crossbeam-epoch v0.9.18
    Checking futf v0.1.5
    Checking parking_lot v0.12.5
    Checking indexmap v2.14.0
   Compiling num-traits v0.2.19
    Checking windows-sys v0.61.2
    Checking jiff-tzdb v0.1.6
    Checking futures-core v0.3.32
    Checking bitflags v2.13.0
   Compiling zstd-sys v2.0.16+zstd.1.5.7
   Compiling napi-build v2.3.2
    Checking futures-sink v0.3.32
   Compiling getrandom v0.4.3
    Checking tinyvec_macros v0.1.1
    Checking utf-8 v0.7.6
   Compiling rayon-core v1.13.0
    Checking rand_core v0.10.1
   Compiling thiserror v2.0.18
    Checking tendril v0.4.3
    Checking tinyvec v1.11.0
    Checking futures-channel v0.3.32
    Checking jiff-tzdb-platform v0.1.3
    Checking crossbeam-deque v0.8.6
    Checking phf v0.10.1
   Compiling phf_codegen v0.13.1
    Checking flate2 v1.1.9
   Compiling indexmap v1.9.3
   Compiling ppv-lite86 v0.2.21
   Compiling proc-macro2-diagnostics v0.10.1
    Checking futures-task v0.3.32
    Checking powerfmt v0.2.0
    Checking cpufeatures v0.3.0
    Checking itoa v0.4.8
    Checking regex-automata v0.4.14
    Checking utf8parse v0.2.2
    Checking time-core v0.1.9
   Compiling zmij v1.0.21
   Compiling convert_case v0.4.0
    Checking nodrop v0.1.14
    Checking anstyle v1.0.14
    Checking matches v0.1.10
    Checking slab v0.4.12
    Checking bumpalo v3.20.3
    Checking lazy_static v1.5.0
    Checking num-conv v0.2.2
   Compiling rand_chacha v0.3.1
   Compiling rand_chacha v0.2.2
    Checking deranged v0.5.8
    Checking once_cell v1.21.4
    Checking once_cell_polyfill v1.70.2
   Compiling zstd-safe v7.2.4
   Compiling rand v0.7.3
    Checking futures-io v0.3.32
    Checking zopfli v0.8.3
   Compiling rand v0.8.6
    Checking anstyle-wincon v3.0.11
    Checking anstyle-parse v1.0.0
    Checking servo_arc v0.1.1
    Checking chacha20 v0.10.0
   Compiling synstructure v0.13.2
   Compiling selectors v0.37.0
    Checking jiff v0.2.29
    Checking unicode-normalization v0.1.25
    Checking anstyle-query v1.1.5
    Checking time v0.3.51
    Checking rand_core v0.9.5
    Checking aes v0.8.4
    Checking fxhash v0.2.1
    Checking is_terminal_polyfill v1.70.2
   Compiling phf_generator v0.10.0
   Compiling phf_generator v0.8.0
   Compiling phf_generator v0.11.3
   Compiling phf_codegen v0.10.0
   Compiling phf_codegen v0.8.0
   Compiling string_cache_codegen v0.5.4
    Checking hashbrown v0.12.3
    Checking either v1.16.0
    Checking crc-catalog v2.5.0
    Checking colorchoice v1.0.5
   Compiling selectors v0.22.0
    Checking thin-slice v0.1.1
    Checking bitflags v1.3.2
    Checking regex v1.12.4
   Compiling unicode-segmentation v1.13.3
    Checking unicode-bidi v0.3.18
    Checking rustc-hash v2.1.2
   Compiling anyhow v1.0.102
   Compiling serde_json v1.0.150
    Checking unicode-properties v0.1.4
    Checking percent-encoding v2.3.2
   Compiling markup5ever v0.11.0
   Compiling thiserror v1.0.69
    Checking form_urlencoded v1.2.2
    Checking env_filter v1.0.1
   Compiling convert_case v0.11.0
    Checking stringprep v0.1.5
    Checking rayon v1.12.0
    Checking crc v3.4.0
   Compiling cssparser v0.27.2
   Compiling phf_macros v0.8.0
   Compiling html5ever v0.26.0
   Compiling zerofrom-derive v0.1.7
   Compiling yoke-derive v0.8.2
   Compiling zerovec-derive v0.11.3
   Compiling displaydoc v0.2.6
   Compiling serde_derive v1.0.228
   Compiling cssparser-macros v0.6.1
   Compiling thiserror-impl v2.0.18
   Compiling futures-macro v0.3.32
   Compiling phf_macros v0.13.1
   Compiling derive_more-impl v2.1.1
    Checking phf v0.13.1
   Compiling derive_more v0.99.20
    Checking rand_chacha v0.9.0
   Compiling zeroize_derive v1.5.0
   Compiling thiserror-impl v1.0.69
    Checking futures-util v0.3.32
    Checking zerofrom v0.1.8
    Checking phf v0.8.0
    Checking derive_more v2.1.1
    Checking cssparser v0.36.0
    Checking anstream v1.0.0
    Checking rand v0.10.1
    Checking tracing-core v0.1.36
   Compiling napi v3.9.3
    Checking yoke v0.8.3
    Checking ecb v0.1.2
    Checking cbc v0.1.2
    Checking md-5 v0.10.6
    Checking hmac v0.12.1
    Checking nom v8.0.0
    Checking zerovec v0.11.6
    Checking zerotrie v0.2.4
    Checking libloading v0.9.0
    Checking servo_arc v0.4.3
   Compiling semver v1.0.28
    Checking weezl v0.1.12
    Checking typed-path v0.12.3
    Checking libbz2-rs-sys v0.2.5
    Checking ttf-parser v0.25.1
    Checking rangemap v1.7.1
    Checking regex-automata v0.1.10
    Checking minimal-lexical v0.2.1
    Checking debug_unsafe v0.1.4
   Compiling napi-derive-backend v5.0.4
    Checking napi-sys v3.2.2
    Checking atoi_simd v0.17.0
    Checking bstr v0.2.17
    Checking bzip2 v0.6.1
    Checking tinystr v0.8.3
    Checking potential_utf v0.1.5
    Checking zeroize v1.9.0
    Checking pbkdf2 v0.12.2
    Checking nom v7.1.3
    Checking env_logger v0.11.10
    Checking icu_collections v2.2.0
    Checking zstd v0.13.3
    Checking icu_locale_core v2.2.0
   Compiling maud_macros v0.27.0
    Checking rand v0.9.4
    Checking lzma-rust2 v0.13.0
   Compiling tracing-attributes v0.1.31
    Checking string_cache v0.8.9
    Checking chrono v0.4.45
    Checking sharded-slab v0.1.7
   Compiling firecrawl_rs v0.1.0 (apps\api\native)
    Checking codepage v0.1.2
    Checking zip v7.2.0
    Checking quick-xml v0.39.4
    Checking tokio v1.52.3
    Checking sha1 v0.10.6
    Checking thread_local v1.1.9
    Checking hex v0.4.3
    Checking mime v0.3.17
    Checking fast-float2 v0.2.3
    Checking icu_provider v2.2.0
   Compiling ctor v1.0.7
    Checking constant_time_eq v0.3.1
    Checking deflate64 v0.1.12
    Checking nohash-hasher v0.2.0
    Checking base64 v0.22.1
    Checking uuid v1.23.3
    Checking fnv v1.0.7
    Checking psl-types v2.0.11
    Checking ppmd-rust v1.4.0
    Checking calamine v0.34.0 (https://github.com/firecrawl/calamine?branch=fc-prod#47ad71f8)
    Checking lol_html v2.9.0
    Checking tracing-subscriber v0.3.23
    Checking psl v2.1.214
    Checking icu_properties v2.2.0
    Checking icu_normalizer v2.2.0
    Checking maud v0.27.0
    Checking tracing v0.1.44
    Checking roxmltree v0.20.0
    Checking lopdf v0.41.0
    Checking cfb v0.10.0
    Checking strsim v0.11.1
   Compiling napi-derive v3.5.6
    Checking zip v5.1.1
    Checking kuchikiki v0.8.2
    Checking futures-executor v0.3.32
    Checking idna_adapter v1.2.2
    Checking futures v0.3.32
    Checking idna v1.1.0
    Checking nodesig v1.0.0 (https://github.com/firecrawl/nodesig#b8ebc4a7)
    Checking url v2.5.8
    Checking texting_robots v0.2.2
    Checking pdf-inspector v0.1.3
error: could not compile `firecrawl_rs` (lib) due to 1 previous error stdout={"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.4","manifest_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"cfg_if","src_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\src\\l
[OK] Sandbox: processo efemero concluido command=oxlint pid=32524 exit_code=0 stdout_bytes=10891 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v2
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/controllers/v2 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v2 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/db cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\db concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=46000 exit_code=0 stdout_bytes=1327 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/controllers::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib/branding cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\branding concurrency_limit=3 in_flight=3
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\native
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=apps/api/native cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib/deep-research cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deep-research concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=15224 exit_code=0 stdout_bytes=5995 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\branding
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib/branding cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\branding available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib/deterministicJson cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deterministicJson concurrency_limit=3 in_flight=3
[ERR] Sandbox: processo efemero concluido command=oxlint pid=42932 exit_code=0 stdout_bytes=191 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\db
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/db cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\db available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib/extract cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\extract concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=16984 exit_code=0 stdout_bytes=661 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deep-research
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib/deep-research cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deep-research available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib/generate-llmstxt cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\generate-llmstxt concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=150
[OK] Sandbox: processo efemero concluido command=oxlint pid=43352 exit_code=0 stdout_bytes=966 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deterministicJson
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib/deterministicJson cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deterministicJson available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib/scrape-interact cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\scrape-interact concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=32492 exit_code=0 stdout_bytes=18269 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\extract
[OK] Sandbox: processo efemero concluido command=oxlint pid=35844 exit_code=0 stdout_bytes=1565 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\generate-llmstxt
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib/generate-llmstxt cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\generate-llmstxt available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib/extract cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\extract available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib::files-02 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=37116 exit_code=0 stdout_bytes=1675 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\scrape-interact
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib/scrape-interact cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\scrape-interact available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib::files-03 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=27152 exit_code=0 stdout_bytes=1873 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[OK] Sandbox: processo efemero concluido command=oxlint pid=43812 exit_code=0 stdout_bytes=7455 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib::files-02 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/lib::files-04 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/main cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\main concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=17064 exit_code=0 stdout_bytes=6087 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib::files-03 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/routes cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\routes concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=2160 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\main
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/main cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\main available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/scraper/WebScraper cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\WebScraper concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=3860 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/lib::files-04 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/scraper/scrapeURL cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\scrapeURL concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=31468 exit_code=0 stdout_bytes=1005 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\routes
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/routes cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\routes available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/search cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\search concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=41416 exit_code=0 stdout_bytes=2762 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\WebScraper
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/scraper/WebScraper cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\WebScraper available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/scraper/crawler cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\crawler concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=30744 exit_code=0 stdout_bytes=27768 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\scrapeURL
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/scraper/scrapeURL cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\scrapeURL available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/services/alerts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\alerts concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=27664 exit_code=0 stdout_bytes=3740 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\search
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/search cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\search available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/services/autumn cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\autumn concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=43348 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\crawler
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/scraper/crawler cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\crawler available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src/services::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=33164 exit_code=0 stdout_bytes=192 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\alerts
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/services/alerts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\alerts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/api/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=18232 exit_code=0 stdout_bytes=1541 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\autumn
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/services/autumn cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\autumn available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/js-sdk/firecrawl/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\js-sdk\firecrawl\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=41512 exit_code=0 stdout_bytes=6213 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src/services::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/playwright-service-ts cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\playwright-service-ts concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=19648 exit_code=0 stdout_bytes=7790 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/api/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/test-site/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-site\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=27080 exit_code=0 stdout_bytes=16874 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\js-sdk\firecrawl\src
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/js-sdk/firecrawl/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\js-sdk\firecrawl\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/test-suite cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-suite concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=30692 exit_code=0 stdout_bytes=1437 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\playwright-service-ts
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/playwright-service-ts cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\playwright-service-ts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=apps/ui/ingestion-ui/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\ui\ingestion-ui\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=23792 exit_code=0 stdout_bytes=559 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-site\src
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/test-site/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-site\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=examples/scrape_and_analyze_airbnb_data_e2b cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\examples\scrape_and_analyze_airbnb_data_e2b concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=30260 exit_code=0 stdout_bytes=548 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-suite
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/test-suite cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\test-suite available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=govulncheck scope=apps/api/sharedLibs/go-html-to-md cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\sharedLibs\go-html-to-md concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=17716 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\ui\ingestion-ui\src
[OK] Sandbox: processo efemero concluido command=oxlint pid=42696 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\examples\scrape_and_analyze_airbnb_data_e2b
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=apps/ui/ingestion-ui/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\ui\ingestion-ui\src available_permits=0
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=examples/scrape_and_analyze_airbnb_data_e2b cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\examples\scrape_and_analyze_airbnb_data_e2b available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=govulncheck scope=apps/go-html-to-md-service cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\go-html-to-md-service concurrency_limit=3 in_flight=3
[PROC] SAST monorepo: permissão adquirida blade=ruff scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=ruff pid=28972 exit_code=1 stdout_bytes=228521 stderr_bytes=16127 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl
[PROC] SAST monorepo: sub-scan concluído blade=ruff scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=bandit scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=govulncheck pid=10888 exit_code=2 stdout_bytes=289 stderr_bytes=55 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\sharedLibs\go-html-to-md
[ERR] Sidecar terminou com exit code nao zero binary=govulncheck exit_code=2 stderr=govulncheck: no packages matched the provided patterns stdout={   "config": {     "protocol_version": "v1.0.0",     "scanner_name": "govulncheck",     "scanner_version": "v1.4.0",     "db": "https://vuln.go.dev",     "db_last_modified": "2026-06-16T23:55:18Z",     "go_version": "go1.26.4",     "scan_level": "symbol",     "scan_mode": "source"   } }
[PROC] SAST monorepo: sub-scan concluído blade=govulncheck scope=apps/api/sharedLibs/go-html-to-md cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\sharedLibs\go-html-to-md available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=govulncheck scope=apps/go-sdk cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\go-sdk concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=bandit pid=2292 exit_code=1 stdout_bytes=1122947 stderr_bytes=156 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl
[PROC] SAST monorepo: sub-scan concluído blade=bandit scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=.github/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\.github\scripts concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=795 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=govulncheck pid=27940 exit_code=0 stdout_bytes=367820 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\go-sdk
[PROC] SAST monorepo: sub-scan concluído blade=govulncheck scope=apps/go-sdk cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\go-sdk available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/native/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=180
[OK] Sandbox: processo efemero concluido command=govulncheck pid=17076 exit_code=0 stdout_bytes=492825 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\go-html-to-md-service
[PROC] SAST monorepo: sub-scan concluído blade=govulncheck scope=apps/go-html-to-md-service cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\go-html-to-md-service available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\scripts concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=210
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=240
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=270
[OK] Sandbox: processo efemero concluido command=opengrep pid=35288 exit_code=0 stdout_bytes=879 stderr_bytes=1873 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\scripts
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\scripts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/controllers/v0 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v0 concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=11292 exit_code=0 stdout_bytes=53685 stderr_bytes=1449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\.github\scripts
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=.github/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\.github\scripts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/controllers/v1 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v1 concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=16692 exit_code=0 stdout_bytes=543056 stderr_bytes=1580 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/native/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\native\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/controllers/v2 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v2 concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=300
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=330
[OK] Sandbox: processo efemero concluido command=opengrep pid=39628 exit_code=0 stdout_bytes=68301 stderr_bytes=1229 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v0
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/controllers/v0 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v0 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/controllers::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=360
[OK] Sandbox: processo efemero concluido command=opengrep pid=23588 exit_code=0 stdout_bytes=210274 stderr_bytes=1410 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v1
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/controllers/v1 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v1 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/db cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\db concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=390
[OK] Sandbox: processo efemero concluido command=opengrep pid=18828 exit_code=0 stdout_bytes=61045 stderr_bytes=1408 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/controllers::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib/branding cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\branding concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=420
[OK] Sandbox: processo efemero concluido command=opengrep pid=12076 exit_code=0 stdout_bytes=2573 stderr_bytes=1225 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\db
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/db cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\db available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib/deep-research cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deep-research concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=18928 exit_code=0 stdout_bytes=631008 stderr_bytes=1508 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v2
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/controllers/v2 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\controllers\v2 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib/deterministicJson cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deterministicJson concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=450
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=480
[OK] Sandbox: processo efemero concluido command=opengrep pid=26592 exit_code=0 stdout_bytes=27546 stderr_bytes=1327 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deep-research
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib/deep-research cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deep-research available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib/extract cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\extract concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[ERR] Sandbox: processo efemero concluido command=opengrep pid=45456 exit_code=0 stdout_bytes=73035 stderr_bytes=1429 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deterministicJson
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib/deterministicJson cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\deterministicJson available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib/generate-llmstxt cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\generate-llmstxt concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=510
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=540
[OK] Sandbox: processo efemero concluido command=opengrep pid=22764 exit_code=0 stdout_bytes=242790 stderr_bytes=1477 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\branding
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib/branding cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\branding available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib/scrape-interact cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\scrape-interact concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=10768 exit_code=0 stdout_bytes=14471 stderr_bytes=1226 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\generate-llmstxt
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib/generate-llmstxt cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\generate-llmstxt available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=570
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=600
[OK] Sandbox: processo efemero concluido command=opengrep pid=14932 exit_code=0 stdout_bytes=61330 stderr_bytes=1227 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\scrape-interact
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib/scrape-interact cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\scrape-interact available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib::files-02 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=31996 exit_code=0 stdout_bytes=306630 stderr_bytes=1411 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\extract
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib/extract cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib\extract available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib::files-03 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=630
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=660
[OK] Sandbox: processo efemero concluido command=opengrep pid=2296 exit_code=0 stdout_bytes=98057 stderr_bytes=1508 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/lib::files-04 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=690
[OK] Sandbox: processo efemero concluido command=opengrep pid=44108 exit_code=0 stdout_bytes=111742 stderr_bytes=1411 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib::files-03 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/main cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\main concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=23764 exit_code=0 stdout_bytes=167030 stderr_bytes=1411 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib::files-02 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/scraper/WebScraper cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\WebScraper concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=720
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=750
[OK] Sandbox: processo efemero concluido command=opengrep pid=29048 exit_code=0 stdout_bytes=6609 stderr_bytes=1224 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\main
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/main cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\main available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/scraper/crawler cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\crawler concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=30020 exit_code=0 stdout_bytes=16113 stderr_bytes=1410 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/lib::files-04 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\lib available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/scraper/scrapeURL cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\scrapeURL concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=780
[OK] Sandbox: processo efemero concluido command=opengrep pid=43584 exit_code=0 stdout_bytes=155433 stderr_bytes=1410 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\WebScraper
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/scraper/WebScraper cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\WebScraper available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/search cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\search concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=810
[OK] Sandbox: processo efemero concluido command=opengrep pid=40688 exit_code=0 stdout_bytes=10574 stderr_bytes=1224 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\crawler
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/scraper/crawler cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\crawler available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/services/alerts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\alerts concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=840
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=870
[OK] Sandbox: processo efemero concluido command=opengrep pid=11988 exit_code=0 stdout_bytes=46424 stderr_bytes=1229 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\search
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/search cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\search available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/services::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=13744 exit_code=0 stdout_bytes=129 stderr_bytes=1224 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\alerts
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/services/alerts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services\alerts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=900
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=930
[OK] Sandbox: processo efemero concluido command=opengrep pid=27916 exit_code=0 stdout_bytes=197253 stderr_bytes=1410 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/services::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\services available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=apps/api/src/routes cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\routes concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\.soda_semgrep\firecrawl\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=41540 exit_code=0 stdout_bytes=935639 stderr_bytes=1631 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\scrapeURL
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/scraper/scrapeURL cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\scraper\scrapeURL available_permits=1
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=960
[OK] Sandbox: processo efemero concluido command=opengrep pid=30048 exit_code=0 stdout_bytes=40386 stderr_bytes=1227 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\routes
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src/routes cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src\routes available_permits=2
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=990
[OK] Sandbox: processo efemero concluido command=opengrep pid=38148 exit_code=0 stdout_bytes=859245 stderr_bytes=1756 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=apps/api/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl\apps\api\src available_permits=3
[FINOPS] N11: roteador poliglota de SAST concluido repo_id=mendableai/firecrawl elapsed_ms=972010 unsafe_hotspots_bytes=252645 health_report_bytes=320609
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_06_unsafe_hotspots payload_bytes=252645
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_08_health_report payload_bytes=320609
[PROC] N10: Finalizando coleta de metadados comunitarios repo_id=mendableai/firecrawl
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_09_community_meta payload_bytes=2342
[PROC] N11: Extraindo blob_10_soda_canon_context repo_id=mendableai/firecrawl
[PROC] Blob gerado repo_id=mendableai/firecrawl artifact_type=blob_10_soda_canon_context payload_bytes=4648
[PROC] N12: Persistindo pacote RAW no SQLite repo_id=mendableai/firecrawl blobs_count=11 total_payload_bytes=1356453
[OK] N12: Persistencia do pacote RAW concluida repo_id=mendableai/firecrawl
[OK] N13: pipeline_core retornou; iniciando teardown repo_id=mendableai/firecrawl is_ok=true
[PROC] N13: PurgeGuard iniciando limpeza atomica (Sandbox + TempWorkspace) repo_id=mendableai/firecrawl
[PROC] PurgeGuard: Iniciando limpeza atômica de recursos
[PROC] PurgeGuard: SandboxHandle descartado
[PROC] RamdiskHandle: iniciando teardown ProjFS path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100 projected_roots=1
[FINOPS] RamdiskHandle: virtualization root delegada para delecao externa path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100\repos\mendableai\firecrawl elapsed_ms=21
[FINOPS] RamdiskHandle: cleanup explicito concluido com delecao externa não-bloqueante path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782280236050848100 elapsed_ms=35
[PROC] RamdiskGuard: cleanup ja delegado externamente; Drop nao repetira a remocao
[PROC] F0: heartbeat repo_id=mendableai/firecrawl elapsed_s=1020
[PROC] PurgeGuard: RamdiskHandle descartado
[PROC] N13: Teardown finalizado; retornando ao CLI repo_id=mendableai/firecrawl
[FINOPS] F0: concluído repo_id=mendableai/firecrawl row_number=306 report=Z:\genesis_mc\.soda_scratchpad\reports\_ETL_REPORT_mendableai_firecrawl.txt elapsed_ms=1025191
[PROC] F0(batch): iniciando repo_id=huggingface/candle row_number=365 idx=2 total=5
[PROC] Iniciando HarvesterOrchestrator (N14) url=https://github.com/huggingface/candle repo_id=huggingface/candle
[PROC] N1: Alocando workspace efemero da F0 repo_id=huggingface/candle requested_mb=256
[PROC] N1: Workspace efemero pronto repo_id=huggingface/candle workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900
[PROC] N2: Iniciando clone blobless repo_id=huggingface/candle url=https://github.com/huggingface/candle
[PROC] Preparando workspace efemero do clone url=https://github.com/huggingface/candle workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900 dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[PROC] ProjFS: consultando metadados do repositório GitHub url=https://api.github.com/repos/huggingface/candle
[PROC] ProjFS: consultando release mais recente do repositório url=https://api.github.com/repos/huggingface/candle/releases/latest
[PROC] ProjFS: consultando SHA do commit HEAD url=https://api.github.com/repos/huggingface/candle/commits?sha=main&per_page=1
[PROC] ProjFS: baixando snapshot compactado do repositório url=https://api.github.com/repos/huggingface/candle/zipball/main default_branch=main selected_branch=main
[PROC] ProjFS: snapshot ZIP recebido em memória archive_bytes=6854729
[FINOPS] Clone virtual via ProjFS concluido url=https://github.com/huggingface/candle dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle projected_files=1133 projected_bytes=15212842 elapsed_ms=3086
[OK] N2: Clone blobless concluido repo_id=huggingface/candle repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[PROC] N3: Criando sandbox efemero repo_id=huggingface/candle repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[PROC] N3: Sandbox pronto repo_id=huggingface/candle
[PROC] N4: Detectando stack do repositório repo_id=huggingface/candle
[PROC] N4: Stack detectada repo_id=huggingface/candle profile=Mixed([Rust, CCpp, NodeJS, Python])
[PROC] N5: Roteando tarefas de extração repo_id=huggingface/candle
[PROC] N5: Tarefas roteadas repo_id=huggingface/candle tasks=[RunNativeAstParser, DiscoverTests, ExtractManifests, RunStaticAnalysis, FetchCommunityMeta, ExtractOpsBlueprint, RunOxc]
[PROC] N10: Iniciando coleta concorrente de metadados comunitarios repo_id=huggingface/candle
[PROC] ast-native: iniciando extração estrutural repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[PROC] ast-native: artefatos normalizados repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle repo_outline_bytes=298836 architecture_map_bytes=24400 health_report_bytes=232
[FINOPS] N6: parser AST nativo concluido repo_id=huggingface/candle elapsed_ms=17594 repo_outline_bytes=298836 architecture_map_bytes=24400
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_04_repo_outline payload_bytes=298836
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_05_architecture_map payload_bytes=24400
[PROC] N7: Extraindo blob_01_promessa_readme repo_id=huggingface/candle
[PROC] Tentando ler arquivo para artefato artifact_type=blob_01_promessa_readme candidate=README.md abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\README.md
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_01_promessa_readme payload_bytes=8367
[PROC] N8: Extraindo blob_02_dependency_manifest repo_id=huggingface/candle
[OK] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-book/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-book\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-core/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-core\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-datasets/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-datasets\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-examples/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-examples\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-flash-attn/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-flash-attn-v3/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn-v3\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-kernels/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-kernels\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-metal-kernels/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-metal-kernels\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-nn/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-nn\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-onnx/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-onnx\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-pyo3/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-pyo3\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-pyo3/pyproject.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-pyo3\pyproject.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-transformers/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-ug/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-ug\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/bert/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\bert\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/blip/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\blip\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/chat-template/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\chat-template\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/llama2-c/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\llama2-c\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/moondream/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\moondream\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/phi/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\phi\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/quant-qwen3/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\quant-qwen3\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/segment-anything/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\segment-anything\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/t5/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\t5\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/whisper/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\whisper\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-examples/yolo/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\yolo\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=candle-wasm-tests/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-tests\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\Cargo.toml
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=tensor-tools/Cargo.toml abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\tensor-tools\Cargo.toml
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_02_dependency_manifest payload_bytes=5715
[PROC] N9: Extraindo blob_07_ops_blueprint repo_id=huggingface/candle
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_07_ops_blueprint payload_bytes=10826
[PROC] N11: Extraindo blob_03_test_intent repo_id=huggingface/candle
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_03_test_intent payload_bytes=13315
[PROC] N11: Extraindo blob_11_ux_contracts repo_id=huggingface/candle
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_11_ux_contracts payload_bytes=30
[OK] SAST monorepo: manifestos detectados repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle manifest_count=27 manifests=[".:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\Cargo.toml", "candle-book:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-book\\Cargo.toml", "candle-core:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-core\\Cargo.toml", "candle-datasets:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-datasets\\Cargo.toml", "candle-examples:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-examples\\Cargo.toml", "candle-flash-attn:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-flash-attn\\Cargo.toml", "candle-flash-attn-v3:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-flash-attn-v3\\Cargo.toml", "candle-kernels:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-kernels\\Cargo.toml", "candle-metal-kernels:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-metal-kernels\\Cargo.toml", "candle-nn:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-nn\\Cargo.toml", "candle-onnx:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-onnx\\Cargo.toml", "candle-pyo3:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-pyo3\\Cargo.toml", "candle-transformers:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-transformers\\Cargo.toml", "candle-ug:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-ug\\Cargo.toml", "candle-wasm-examples/bert:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\bert\\Cargo.toml", "candle-wasm-examples/blip:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\blip\\Cargo.toml", "candle-wasm-examples/chat-template:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\chat-template\\Cargo.toml", "candle-wasm-examples/llama2-c:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\llama2-c\\Cargo.toml", "candle-wasm-examples/moondream:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\moondream\\Cargo.toml", "candle-wasm-examples/phi:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\phi\\Cargo.toml", "candle-wasm-examples/quant-qwen3:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\quant-qwen3\\Cargo.toml", "candle-wasm-examples/segment-anything:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\segment-anything\\Cargo.toml", "candle-wasm-examples/t5:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\t5\\Cargo.toml", "candle-wasm-examples/whisper:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\whisper\\Cargo.toml", "candle-wasm-examples/yolo:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-examples\\yolo\\Cargo.toml", "candle-wasm-tests:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\candle-wasm-tests\\Cargo.toml", "tensor-tools:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782281264489518900\\repos\\huggingface\\candle\\tensor-tools\\Cargo.toml"] concurrency_limit=3
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle concurrency_limit=3 in_flight=1
[OK] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-book cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-book concurrency_limit=3 in_flight=2
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-core cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-core concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=25680 exit_code=101 stdout_bytes=0 stderr_bytes=327 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-book
[ERR] Sidecar terminou com exit code nao zero binary=cargo exit_code=101 stderr=error: failed to parse manifest at `candle-book\Cargo.toml`

Caused by:
  error inheriting `edition` from workspace root manifest's `workspace.package.edition`

Caused by:
  failed to find a workspace root stdout=
[OK] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-book cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-book available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-datasets cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-datasets concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=30
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=60
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=90
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=120
[OK] Sandbox: processo efemero concluido command=cargo pid=45924 exit_code=0 stdout_bytes=131181 stderr_bytes=5714 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-core
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-core
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-core cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-core available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-examples cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-examples concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=150
[OK] Sandbox: processo efemero concluido command=cargo pid=12136 exit_code=101 stdout_bytes=273071 stderr_bytes=8492 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[ERR] Sidecar terminou com exit code nao zero binary=cargo exit_code=101 stderr=Blocking waiting for file lock on package cache
    Updating crates.io index
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
   Compiling unicode-ident v1.0.24
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.46
    Checking cfg-if v1.0.4
    Checking once_cell v1.21.4
    Checking memchr v2.8.2
    Checking pin-project-lite v0.2.17
   Compiling rustversion v1.0.22
    Checking futures-core v0.3.32
    Checking futures-sink v0.3.32
   Compiling serde_core v1.0.228
    Checking slab v0.4.12
    Checking futures-io v0.3.32
    Checking futures-task v0.3.32
   Compiling autocfg v1.5.1
   Compiling serde v1.0.228
    Checking itoa v1.0.18
   Compiling wasm-bindgen-shared v0.2.122
    Checking windows-link v0.2.1
   Compiling zmij v1.0.21
    Checking futures-channel v0.3.32
   Compiling bumpalo v3.20.3
   Compiling libm v0.2.16
   Compiling version_check v0.9.5
   Compiling serde_json v1.0.150
   Compiling getrandom v0.3.4
    Checking windows-sys v0.61.2
   Compiling num-traits v0.2.19
   Compiling zerocopy v0.8.52
    Checking bitflags v2.13.0
   Compiling find-msvc-tools v0.1.9
   Compiling shlex v2.0.1
   Compiling crossbeam-utils v0.8.21
   Compiling paste v1.0.15
   Compiling libc v0.2.186
   Compiling rayon-core v1.13.0
   Compiling jobserver v0.1.34
    Checking rand_core v0.9.5
    Checking bytes v1.12.0
   Compiling wasm-bindgen v0.2.122
    Checking stable_deref_trait v1.2.1
    Checking either v1.16.0
    Checking reborrow v0.5.5
   Compiling cc v1.2.65
    Checking raw-cpuid v11.6.0
   Compiling syn v2.0.118
   Compiling dyn-stack-macros v0.1.3
   Compiling seq-macro v0.3.6
   Compiling crc32fast v1.5.0
   Compiling fnv v1.0.7
    Checking ryu v1.0.23
    Checking hashbrown v0.17.1
    Checking crossbeam-epoch v0.9.18
   Compiling thiserror v2.0.18
    Checking equivalent v1.0.2
   Compiling pkg-config v0.3.33
    Checking smallvec v1.15.2
   Compiling pulp v0.22.3
    Checking num_cpus v1.17.0
    Checking crossbeam-deque v0.8.6
    Checking pulp-wasm-simd-flag v0.1.1
   Compiling ahash v0.8.12
    Checking tracing-core v0.1.36
    Checking rayon v1.12.0
   Compiling strsim v0.11.1
    Checking indexmap v2.14.0
    Checking log v0.4.33
   Compiling ident_case v1.0.1
    Checking aho-corasick v1.1.4
    Checking regex-syntax v0.8.11
    Checking byteorder v1.5.0
   Compiling onig_sys v69.9.3
   Compiling getrandom v0.4.3
   Compiling thiserror v1.0.69
    Checking bit-vec v0.8.0
    Checking percent-encoding v2.3.2
    Checking minimal-lexical v0.2.1
   Compiling esaxx-rs v0.1.10
    Checking itertools v0.14.0
    Checking memmap2 v0.9.11
    Checking castaway v0.2.4
    Checking fastrand v2.4.1
    Checking bit-set v0.8.0
    Checking unicode-segmentation v1.13.3
    Checking nom v7.1.3
   Compiling macro_rules_attribute-proc_macro v0.2.2
    Checking static_assertions v1.1.0
    Checking base64 v0.13.1
    Checking foldhash v0.2.0
    Checking allocator-api2 v0.2.21
    Checking unicode-normalization-alignments v0.1.12
   Compiling wasm-bindgen-macro-support v0.2.122
   Compiling synstructure v0.13.2
   Compiling darling_core v0.20.11
    Checking unicode_categories v0.1.1
    Checking typed-path v0.12.3
    Checking hashbrown v0.16.1
    Checking form_urlencoded v1.2.2
    Checking getrandom v0.2.17
   Compiling anyhow v1.0.102
    Checking http v1.4.2
   Compiling toml_datetime v0.6.11
    Checking regex-automata v0.4.14
    Checking rayon-cond v0.4.0
    Checking simd-adler32 v0.3.9
    Checking zip v8.6.0
    Checking adler2 v2.0.1
   Compiling winnow v0.7.15
   Compiling syn v1.0.109
   Compiling icu_normalizer_data v2.2.0
    Checking miniz_oxide v0.8.9
   Compiling httparse v1.10.1
    Checking zlib-rs v0.6.4
   Compiling icu_properties_data v2.2.0
   Compiling proc-macro-error-attr v1.0.4
    Checking socket2 v0.6.4
    Checking fancy-regex v0.14.0
   Compiling futures-macro v0.3.32
   Compiling serde_derive v1.0.228
   Compiling zerofrom-derive v0.1.7
   Compiling yoke-derive v0.8.2
   Compiling zerocopy-derive v0.8.52
   Compiling bytemuck_derive v1.10.2
   Compiling thiserror-impl v2.0.18
   Compiling tokio-macros v2.7.0
   Compiling tracing-attributes v0.1.31
   Compiling darling_macro v0.20.11
   Compiling thiserror-impl v1.0.69
    Checking futures-util v0.3.32
   Compiling darling v0.20.11
   Compiling derive_builder_core v0.20.2
    Checking zerofrom v0.1.8
    Checking bytemuck v1.25.0
   Compiling monostate-impl v0.1.18
    Checking macro_rules_attribute v0.2.2
    Checking num-complex v0.4.6
    Checking dyn-stack v0.13.2
    Checking regex v1.12.4
    Checking tempfile v3.27.0
    Checking yoke v0.8.3
   Compiling zerovec-derive v0.11.3
    Checking mio v1.2.1
   Compiling displaydoc v0.2.6
   Compiling derive_builder_macro v0.20.2
   Compiling pin-project-internal v1.1.13
   Compiling wasm-bindgen-macro v0.2.122
    Checking tokio v1.52.3
    Checking futures-executor v0.3.32
    Checking fancy-regex v0.18.0
    Checking futures v0.3.32
    Checking monostate v0.1.18
    Checking pinned v0.1.0
    Checking onig v6.5.3
   Compiling toml_edit v0.22.27
    Checking derive_builder v0.20.2
    Checking flate2 v1.1.9
    Checking pin-project v1.1.13
    Checking http v0.2.12
    Checking tracing v0.1.44
   Compiling proc-macro-error v1.0.4
   Compiling proc-macro-crate v3.3.0
    Checking ppv-lite86 v0.2.21
   Compiling target-lexicon v0.13.5
   Compiling winnow v0.5.40
    Checking tokio-stream v0.1.18
    Checking fdeflate v0.3.7
   Compiling indexmap v1.9.3
   Compiling prettyplease v0.1.25
    Checking spm_precompiled v0.1.4
    Checking rand_chacha v0.9.0
    Checking compact_str v0.9.1
    Checking dary_heap v0.3.9
    Checking safetensors v0.8.0
    Checking serde_urlencoded v0.7.1
    Checking rand v0.9.4
    Checking bincode v1.3.3
    Checking serde_plain v1.0.2
    Checking pxfm v0.1.29
    Checking anymap2 v0.13.0
    Checking zune-core v0.5.1
   Compiling native-tls v0.2.18
    Checking png v0.18.1
    Checking rand_distr v0.5.1
    Checking tokenizers v0.22.2
   Compiling zerovec v0.11.6
    Checking zune-jpeg v0.5.15
   Compiling ring v0.17.14
   Compiling gloo-worker-macros v0.2.0
    Checking byteorder-lite v0.1.0
    Checking half v2.7.1
    Checking hashbrown v0.12.3
    Checking js-sys v0.3.99
    Checking console_error_panic_hook v0.1.7
   Compiling prettyplease v0.2.37
   Compiling boolinator v2.4.0
    Checking prokio v0.1.0
    Checking gemm-common v0.19.0
    Checking float8 v0.7.0
   Compiling implicit-clone-derive v0.1.2
    Checking gemm-f32 v0.19.0
    Checking gemm-c64 v0.19.0
    Checking gemm-f64 v0.19.0
    Checking gemm-c32 v0.19.0
   Compiling toml_edit v0.19.15
    Checking implicit-clone v0.3.10
    Checking gemm-f16 v0.19.0
   Compiling yew-macro v0.20.0
   Compiling tinystr v0.8.3
   Compiling pyo3-build-config v0.27.2
    Checking tokise v0.2.1
    Checking num-integer v0.1.46
   Compiling winapi v0.3.9
   Compiling litemap v0.8.2
    Checking gemm v0.19.0
   Compiling writeable v0.6.3
   Compiling potential_utf v0.1.5
   Compiling zerotrie v0.2.4
   Compiling icu_locale_core v2.2.0
   Compiling utf8_iter v1.0.4
    Checking moxcms v0.8.1
    Checking candle-core v0.10.2 (candle-core)
   Compiling zeroize v1.9.0
   Compiling icu_collections v2.2.0
    Checking num-bigint v0.4.6
    Checking implicit-clone v0.6.0
   Compiling yew-macro v0.23.0
   Compiling icu_provider v2.2.0
   Compiling proc-macro-crate v1.3.1
   Compiling pulp v0.21.5
error: failed to run custom build command for `pyo3-build-config v0.27.2`

Caused by:
  process didn't exit successfully: `Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle\debug\build\pyo3-build-config-ab9f0a0d7898bb73\build-script-build` (exit code: 1)
  --- stdout
  cargo:rerun-if-env-changed=PYO3_CONFIG_FILE
  cargo:rerun-if-env-changed=PYO3_NO_PYTHON
  cargo:rerun-if-env-changed=PYO3_ENVIRONMENT_SIGNATURE
  cargo:rerun-if-env-changed=PYO3_PYTHON
  cargo:rerun-if-env-changed=VIRTUAL_ENV
  cargo:rerun-if-env-changed=CONDA_PREFIX
  cargo:rerun-if-env-changed=PATH

  --- stderr
  error: cannot set a minimum Python version 3.13 higher than the interpreter version 3.12 (the minimum Python version is implied by the abi3-py313 feature)
warning: build failed, waiting for other jobs to finish... stdout={"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.4","manifest_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"cfg_if","src_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\src\\l
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-flash-attn-v3 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn-v3 concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=180
[OK] Sandbox: processo efemero concluido command=cargo pid=13916 exit_code=0 stdout_bytes=262679 stderr_bytes=8949 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-datasets
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-datasets
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-datasets cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-datasets available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-flash-attn cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=210
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=240
[OK] Sandbox: processo efemero concluido command=cargo pid=31628 exit_code=101 stdout_bytes=166023 stderr_bytes=10713 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn-v3
[ERR] Sidecar terminou com exit code nao zero binary=cargo exit_code=101 stderr=Updating crates.io index
     Locking 207 packages to latest compatible versions
      Adding generic-array v0.14.7 (available: v0.14.9)
      Adding rstest v0.23.0 (available: v0.26.1)
   Compiling proc-macro2 v1.0.106
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.46
   Compiling version_check v0.9.5
   Compiling crossbeam-utils v0.8.21
    Checking cfg-if v1.0.4
   Compiling rayon-core v1.13.0
   Compiling zerocopy v0.8.52
   Compiling getrandom v0.3.4
   Compiling libm v0.2.16
   Compiling autocfg v1.5.1
   Compiling zmij v1.0.21
   Compiling serde_core v1.0.228
   Compiling paste v1.0.15
    Checking once_cell v1.21.4
   Compiling typenum v1.20.1
    Checking bitflags v2.13.0
    Checking either v1.16.0
   Compiling windows-link v0.2.1
   Compiling thiserror v2.0.18
   Compiling windows-sys v0.61.2
   Compiling generic-array v0.14.7
   Compiling num-traits v0.2.19
   Compiling pulp v0.22.3
   Compiling winapi v0.3.9
    Checking memchr v2.8.2
    Checking raw-cpuid v11.6.0
   Compiling serde_json v1.0.150
    Checking pulp-wasm-simd-flag v0.1.1
    Checking rand_core v0.9.5
   Compiling block-buffer v0.10.4
    Checking crossbeam-epoch v0.9.18
   Compiling crypto-common v0.1.7
   Compiling syn v2.0.118
   Compiling winapi-util v0.1.11
    Checking reborrow v0.5.5
    Checking crossbeam-deque v0.8.6
   Compiling serde v1.0.228
   Compiling anyhow v1.0.102
   Compiling dyn-stack-macros v0.1.3
   Compiling same-file v1.0.6
   Compiling digest v0.10.7
    Checking rayon v1.12.0
   Compiling env_home v0.1.0
   Compiling seq-macro v0.3.6
   Compiling ident_case v1.0.1
   Compiling cpufeatures v0.2.17
   Compiling fnv v1.0.7
   Compiling itoa v1.0.18
   Compiling strsim v0.11.1
   Compiling winsafe v0.0.19
   Compiling walkdir v2.5.0
   Compiling fs2 v0.4.3
   Compiling glob v0.3.3
   Compiling shlex v2.0.1
   Compiling zerocopy-derive v0.8.52
   Compiling serde_derive v1.0.228
   Compiling bytemuck_derive v1.10.2
   Compiling thiserror-impl v2.0.18
   Compiling sha2 v0.10.9
   Compiling num_cpus v1.17.0
   Compiling find-msvc-tools v0.1.9
   Compiling darling_core v0.20.11
   Compiling pkg-config v0.3.33
   Compiling rustversion v1.0.22
   Compiling synstructure v0.13.2
   Compiling getrandom v0.4.3
   Compiling libc v0.2.186
   Compiling cc v1.2.65
    Checking equivalent v1.0.2
    Checking aho-corasick v1.1.4
   Compiling ahash v0.8.12
   Compiling esaxx-rs v0.1.10
    Checking minimal-lexical v0.2.1
   Compiling crc32fast v1.5.0
    Checking regex-syntax v0.8.11
    Checking nom v7.1.3
   Compiling zerofrom-derive v0.1.7
   Compiling monostate-impl v0.1.18
    Checking regex-automata v0.4.14
   Compiling onig_sys v69.9.3
    Checking bytemuck v1.25.0
    Checking itertools v0.14.0
    Checking static_assertions v1.1.0
   Compiling which v7.0.3
    Checking castaway v0.2.4
    Checking base64 v0.13.1
    Checking ryu v1.0.23
   Compiling darling_macro v0.20.11
   Compiling macro_rules_attribute-proc_macro v0.2.2
   Compiling cudarc v0.19.8
    Checking num-complex v0.4.6
    Checking dyn-stack v0.13.2
    Checking unicode-segmentation v1.13.3
    Checking foldhash v0.2.0
    Checking hashbrown v0.17.1
    Checking smallvec v1.15.2
    Checking fastrand v2.4.1
    Checking stable_deref_trait v1.2.1
    Checking allocator-api2 v0.2.21
    Checking monostate v0.1.18
    Checking compact_str v0.9.1
    Checking macro_rules_attribute v0.2.2
    Checking unicode-normalization-alignments v0.1.12
    Checking spm_precompiled v0.1.4
    Checking tempfile v3.27.0
   Compiling cudaforge v0.1.6
    Checking dary_heap v0.3.9
    Checking regex v1.12.4
    Checking zerofrom v0.1.8
   Compiling yoke-derive v0.8.2
    Checking libloading v0.9.0
    Checking hashbrown v0.16.1
    Checking typed-path v0.12.3
    Checking log v0.4.33
    Checking unicode_categories v0.1.1
    Checking rayon-cond v0.4.0
    Checking indexmap v2.14.0
    Checking memmap2 v0.9.11
    Checking byteorder v1.5.0
    Checking safetensors v0.8.0
   Compiling darling v0.20.11
   Compiling candle-kernels v0.10.2 (candle-kernels)
   Compiling candle-flash-attn-v3 v0.10.2 (candle-flash-attn-v3)
   Compiling derive_builder_core v0.20.2
    Checking ppv-lite86 v0.2.21
    Checking yoke v0.8.3
    Checking zip v8.6.0
    Checking rand_chacha v0.9.0
    Checking rand v0.9.4
   Compiling derive_builder_macro v0.20.2
error: failed to run custom build command for `candle-flash-attn-v3 v0.10.2 (candle-flash-attn-v3)`

Caused by:
  process didn't exit successfully: `Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-flash-attn-v3\debug\build\candle-flash-attn-v3-080dd51f12237a79\build-script-build` (exit code: 101)
  --- stdout
  cargo:rerun-if-changed=build.rs
  cargo:rerun-if-env-changed=CUDA_COMPUTE_CAP
  cargo:rerun-if-env-changed=CANDLE_NVCC_CCBIN
  cargo:rerun-if-changed=hkernel/flash_api.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_fp16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_bf16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_fp16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_bf16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_fp16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_bf16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_fp16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_bf16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_fp16_gqa2_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_fp16_gqa4_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_fp16_gqa8_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_fp16_gqa16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_fp16_gqa32_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_fp16_gqa2_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_fp16_gqa4_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_fp16_gqa8_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_fp16_gqa16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_fp16_gqa32_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_fp16_gqa2_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_fp16_gqa4_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_fp16_gqa8_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_fp16_gqa16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_fp16_gqa32_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_fp16_gqa2_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_fp16_gqa4_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_fp16_gqa8_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_fp16_gqa16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_fp16_gqa32_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_bf16_gqa2_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_bf16_gqa4_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_bf16_gqa8_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_bf16_gqa16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim64_bf16_gqa32_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_bf16_gqa2_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_bf16_gqa4_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_bf16_gqa8_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_bf16_gqa16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim128_bf16_gqa32_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_bf16_gqa2_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_bf16_gqa4_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_bf16_gqa8_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_bf16_gqa16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim256_bf16_gqa32_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_bf16_gqa2_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_bf16_gqa4_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_bf16_gqa8_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_bf16_gqa16_sm90.cu
  cargo:rerun-if-changed=hkernel/flash_fwd_hdim512_bf16_gqa32_sm90.cu
  cargo:rerun-if-changed=kernels/**.h
  cargo:rerun-if-changed=kernels/**.hpp
  cargo:rerun-if-changed=kernels/**.cpp

  --- stderr

  thread 'main' (23936) panicked at build.rs:164:5:
  Compute capability must be >=90 (90a)
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...
warning: candle-kernels@0.10.2: Compiling 11 of 11 PTX kernels
error: failed to run custom build command for `candle-kernels v0.10.2 (candle-kernels)`

Caused by:
  process didn't exit successfully: `Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-flash-attn-v3\debug\build\candle-kernels-39f3e66d8f50d434\build-script-build` (exit code: 1)
  --- stdout
  cargo::rerun-if-changed=build.rs
  cargo::rerun-if-changed=src/compatibility.cuh
  cargo::rerun-if-changed=src/cuda_utils.cuh
  cargo::rerun-if-changed=src/binary_op_macros.cuh
  cargo:rustc-env=CUDA_INCLUDE_DIR=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.3\include
  cargo:rerun-if-changed=src\affine.cu
  cargo:rerun-if-changed=src\binary.cu
  cargo:rerun-if-changed=src\cast.cu
  cargo:rerun-if-changed=src\conv.cu
  cargo:rerun-if-changed=src\fill.cu
  cargo:rerun-if-changed=src\indexing.cu
  cargo:rerun-if-changed=src\quantized.cu
  cargo:rerun-if-changed=src\reduce.cu
  cargo:rerun-if-changed=src\sort.cu
  cargo:rerun-if-changed=src\ternary.cu
  cargo:rerun-if-changed=src\unary.cu
  cargo:rerun-if-env-changed=CUDA_COMPUTE_CAP
  cargo:rerun-if-env-changed=NVCC_CCBIN
  cargo:warning=Compiling 11 of 11 PTX kernels
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH

  --- stderr
  Error: CompilationFailed { path: "src\\affine.cu", message: "nvcc error:\n\n" } stdout={"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.4","manifest_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"cfg_if","src_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\src\\l
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-flash-attn-v3
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-flash-attn-v3 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn-v3 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-kernels cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-kernels concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=44236 exit_code=101 stdout_bytes=164174 stderr_bytes=13447 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn
[ERR] Sidecar terminou com exit code nao zero binary=cargo exit_code=101 stderr=Updating crates.io index
     Locking 186 packages to latest compatible versions
      Adding generic-array v0.14.7 (available: v0.14.9)
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.46
   Compiling unicode-ident v1.0.24
   Compiling version_check v0.9.5
   Compiling crossbeam-utils v0.8.21
    Checking cfg-if v1.0.4
   Compiling rayon-core v1.13.0
   Compiling getrandom v0.3.4
   Compiling zerocopy v0.8.52
   Compiling autocfg v1.5.1
   Compiling libm v0.2.16
   Compiling zmij v1.0.21
   Compiling serde_core v1.0.228
   Compiling paste v1.0.15
    Checking once_cell v1.21.4
    Checking bitflags v2.13.0
    Checking either v1.16.0
   Compiling typenum v1.20.1
   Compiling windows-link v0.2.1
   Compiling thiserror v2.0.18
   Compiling windows-sys v0.61.2
   Compiling generic-array v0.14.7
   Compiling num-traits v0.2.19
   Compiling pulp v0.22.3
   Compiling winapi v0.3.9
    Checking rand_core v0.9.5
    Checking memchr v2.8.2
    Checking raw-cpuid v11.6.0
   Compiling serde v1.0.228
   Compiling winapi-util v0.1.11
   Compiling dyn-stack-macros v0.1.3
   Compiling serde_json v1.0.150
   Compiling anyhow v1.0.102
    Checking pulp-wasm-simd-flag v0.1.1
    Checking reborrow v0.5.5
   Compiling syn v2.0.118
   Compiling same-file v1.0.6
   Compiling strsim v0.11.1
   Compiling seq-macro v0.3.6
   Compiling itoa v1.0.18
   Compiling fnv v1.0.7
   Compiling ident_case v1.0.1
   Compiling winsafe v0.0.19
   Compiling env_home v0.1.0
   Compiling block-buffer v0.10.4
   Compiling crypto-common v0.1.7
   Compiling cpufeatures v0.2.17
   Compiling walkdir v2.5.0
   Compiling shlex v2.0.1
   Compiling digest v0.10.7
   Compiling num_cpus v1.17.0
   Compiling glob v0.3.3
   Compiling sha2 v0.10.9
   Compiling find-msvc-tools v0.1.9
   Compiling pkg-config v0.3.33
   Compiling rustversion v1.0.22
   Compiling cc v1.2.65
   Compiling getrandom v0.4.3
    Checking equivalent v1.0.2
   Compiling libc v0.2.186
    Checking aho-corasick v1.1.4
   Compiling ahash v0.8.12
   Compiling crc32fast v1.5.0
    Checking regex-syntax v0.8.11
   Compiling onig_sys v69.9.3
    Checking minimal-lexical v0.2.1
   Compiling darling_core v0.20.11
   Compiling synstructure v0.13.2
   Compiling esaxx-rs v0.1.10
   Compiling fs2 v0.4.3
    Checking nom v7.1.3
    Checking crossbeam-epoch v0.9.18
    Checking castaway v0.2.4
    Checking itertools v0.14.0
   Compiling cudarc v0.19.8
    Checking fastrand v2.4.1
    Checking crossbeam-deque v0.8.6
   Compiling macro_rules_attribute-proc_macro v0.2.2
    Checking smallvec v1.15.2
    Checking ryu v1.0.23
    Checking unicode-segmentation v1.13.3
    Checking base64 v0.13.1
    Checking stable_deref_trait v1.2.1
    Checking allocator-api2 v0.2.21
    Checking hashbrown v0.17.1
    Checking foldhash v0.2.0
    Checking static_assertions v1.1.0
    Checking macro_rules_attribute v0.2.2
    Checking rayon v1.12.0
    Checking unicode-normalization-alignments v0.1.12
    Checking regex-automata v0.4.14
   Compiling which v7.0.3
    Checking indexmap v2.14.0
    Checking hashbrown v0.16.1
    Checking libloading v0.9.0
    Checking log v0.4.33
    Checking unicode_categories v0.1.1
    Checking typed-path v0.12.3
    Checking tempfile v3.27.0
    Checking memmap2 v0.9.11
    Checking byteorder v1.5.0
   Compiling zerocopy-derive v0.8.52
   Compiling bytemuck_derive v1.10.2
   Compiling serde_derive v1.0.228
   Compiling thiserror-impl v2.0.18
   Compiling monostate-impl v0.1.18
   Compiling zerofrom-derive v0.1.7
    Checking rayon-cond v0.4.0
   Compiling yoke-derive v0.8.2
   Compiling darling_macro v0.20.11
    Checking zip v8.6.0
    Checking monostate v0.1.18
    Checking regex v1.12.4
    Checking onig v6.5.3
   Compiling darling v0.20.11
   Compiling derive_builder_core v0.20.2
    Checking zerofrom v0.1.8
    Checking bytemuck v1.25.0
    Checking yoke v0.8.3
    Checking num-complex v0.4.6
    Checking dyn-stack v0.13.2
    Checking spm_precompiled v0.1.4
    Checking compact_str v0.9.1
    Checking dary_heap v0.3.9
    Checking safetensors v0.8.0
   Compiling derive_builder_macro v0.20.2
   Compiling cudaforge v0.1.6
    Checking derive_builder v0.20.2
   Compiling candle-kernels v0.10.2 (candle-kernels)
   Compiling candle-flash-attn v0.10.2 (candle-flash-attn)
warning: candle-kernels@0.10.2: Compiling 11 of 11 PTX kernels
error: failed to run custom build command for `candle-kernels v0.10.2 (candle-kernels)`

Caused by:
  process didn't exit successfully: `Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-flash-attn\debug\build\candle-kernels-39f3e66d8f50d434\build-script-build` (exit code: 1)
  --- stdout
  cargo::rerun-if-changed=build.rs
  cargo::rerun-if-changed=src/compatibility.cuh
  cargo::rerun-if-changed=src/cuda_utils.cuh
  cargo::rerun-if-changed=src/binary_op_macros.cuh
  cargo:rustc-env=CUDA_INCLUDE_DIR=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.3\include
  cargo:rerun-if-changed=src\affine.cu
  cargo:rerun-if-changed=src\binary.cu
  cargo:rerun-if-changed=src\cast.cu
  cargo:rerun-if-changed=src\conv.cu
  cargo:rerun-if-changed=src\fill.cu
  cargo:rerun-if-changed=src\indexing.cu
  cargo:rerun-if-changed=src\quantized.cu
  cargo:rerun-if-changed=src\reduce.cu
  cargo:rerun-if-changed=src\sort.cu
  cargo:rerun-if-changed=src\ternary.cu
  cargo:rerun-if-changed=src\unary.cu
  cargo:rerun-if-env-changed=CUDA_COMPUTE_CAP
  cargo:rerun-if-env-changed=NVCC_CCBIN
  cargo:warning=Compiling 11 of 11 PTX kernels
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH

  --- stderr
  Error: CompilationFailed { path: "src\\affine.cu", message: "nvcc error:\n\n" }
warning: build failed, waiting for other jobs to finish...
warning: candle-flash-attn@0.10.2: Using 8 threads for compilation
warning: candle-flash-attn@0.10.2: Using cached cutlass at C:\Users\rosas\.cargo\git\checkouts\cutlass-7d49e6c7e2f8896c
warning: candle-flash-attn@0.10.2: Compiling 37 of 37 kernels
error: failed to run custom build command for `candle-flash-attn v0.10.2 (candle-flash-attn)`

Caused by:
  process didn't exit successfully: `Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-flash-attn\debug\build\candle-flash-attn-8d1e5dfcd94dd789\build-script-build` (exit code: 1)
  --- stdout
  cargo::rerun-if-changed=build.rs
  cargo::rerun-if-changed=kernels/flash_api.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim128_fp16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim160_fp16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim192_fp16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim224_fp16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim256_fp16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim512_fp16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim32_fp16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim64_fp16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim96_fp16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim128_bf16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim160_bf16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim192_bf16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim224_bf16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim256_bf16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim512_bf16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim32_bf16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim64_bf16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim96_bf16_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim128_fp16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim160_fp16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim192_fp16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim224_fp16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim256_fp16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim512_fp16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim32_fp16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim64_fp16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim96_fp16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim128_bf16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim160_bf16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim192_bf16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim224_bf16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim256_bf16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim512_bf16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim32_bf16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim64_bf16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_hdim96_bf16_causal_sm80.cu
  cargo::rerun-if-changed=kernels/flash_fwd_kernel.h
  cargo::rerun-if-changed=kernels/flash_fwd_launch_template.h
  cargo::rerun-if-changed=kernels/flash.h
  cargo::rerun-if-changed=kernels/philox.cuh
  cargo::rerun-if-changed=kernels/softmax.h
  cargo::rerun-if-changed=kernels/utils.h
  cargo::rerun-if-changed=kernels/kernel_traits.h
  cargo::rerun-if-changed=kernels/block_info.h
  cargo::rerun-if-changed=kernels/static_switch.h
  cargo::rerun-if-changed=kernels/hardware_info.h
  cargo:warning=Using 8 threads for compilation
  cargo:rerun-if-changed=kernels/flash_api.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim128_bf16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim128_bf16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim128_fp16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim128_fp16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim160_bf16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim160_bf16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim160_fp16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim160_fp16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim192_bf16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim192_bf16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim192_fp16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim192_fp16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim224_bf16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim224_bf16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim224_fp16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim224_fp16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim256_bf16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim256_bf16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim256_fp16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim256_fp16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim32_bf16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim32_bf16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim32_fp16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim32_fp16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim512_bf16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim512_bf16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim512_fp16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim512_fp16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim64_bf16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim64_bf16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim64_fp16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim64_fp16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim96_bf16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim96_bf16_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim96_fp16_causal_sm80.cu
  cargo:rerun-if-changed=kernels/flash_fwd_hdim96_fp16_sm80.cu
  cargo:rerun-if-env-changed=CUDA_COMPUTE_CAP
  cargo:rerun-if-env-changed=NVCC
  cargo:rerun-if-env-changed=NVCC_CCBIN
  cargo:warning=Using cached cutlass at C:\Users\rosas\.cargo\git\checkouts\cutlass-7d49e6c7e2f8896c
  cargo:warning=Compiling 37 of 37 kernels
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH

  --- stderr
  Error: CompilationFailed { path: "kernels/flash_api.cu", message: "nvcc error:\n\n" } stdout={"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.4","manifest_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"cfg_if","src_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\src\\l
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-flash-attn
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-flash-attn cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-metal-kernels cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-metal-kernels concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=270
[OK] Sandbox: processo efemero concluido command=cargo pid=22772 exit_code=101 stdout_bytes=27234 stderr_bytes=878 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-metal-kernels
[ERR] Sidecar terminou com exit code nao zero binary=cargo exit_code=101 stderr=Updating crates.io index
     Locking 55 packages to latest compatible versions
      Adding rand v0.9.4 (available: v0.10.1)
      Adding rand_distr v0.5.1 (available: v0.6.0)
   Compiling proc-macro2 v1.0.106
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.46
   Compiling objc2 v0.6.4
   Compiling zerocopy v0.8.52
   Compiling getrandom v0.3.4
    Checking objc2-encode v4.1.0
    Checking cfg-if v1.0.4
   Compiling autocfg v1.5.1
   Compiling libm v0.2.16
   Compiling libc v0.2.186
    Checking bitflags v2.13.0
    Checking once_cell v1.21.4
   Compiling thiserror v2.0.18
    Checking pin-project-lite v0.2.17
    Checking tracing-core v0.1.36
   Compiling num-traits v0.2.19
    Checking rand_core v0.9.5
   Compiling syn v2.0.118
error: could not compile `objc2` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish... stdout={"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.4","manifest_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"cfg_if","src_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\src\\l
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-metal-kernels
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-metal-kernels cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-metal-kernels available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-onnx cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-onnx concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=41272 exit_code=101 stdout_bytes=54686 stderr_bytes=3444 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-kernels
[ERR] Sidecar terminou com exit code nao zero binary=cargo exit_code=101 stderr=Updating crates.io index
     Locking 50 packages to latest compatible versions
      Adding generic-array v0.14.7 (available: v0.14.9)
   Compiling proc-macro2 v1.0.106
   Compiling version_check v0.9.5
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.46
   Compiling crossbeam-utils v0.8.21
   Compiling typenum v1.20.1
   Compiling windows-link v0.2.1
   Compiling serde_core v1.0.228
   Compiling rayon-core v1.13.0
   Compiling zmij v1.0.21
   Compiling winapi v0.3.9
   Compiling either v1.16.0
   Compiling thiserror v2.0.18
   Compiling serde_json v1.0.150
   Compiling anyhow v1.0.102
   Compiling serde v1.0.228
   Compiling windows-sys v0.61.2
   Compiling memchr v2.8.2
   Compiling cpufeatures v0.2.17
   Compiling generic-array v0.14.7
   Compiling cfg-if v1.0.4
   Compiling winsafe v0.0.19
   Compiling itoa v1.0.18
   Compiling env_home v0.1.0
   Compiling glob v0.3.3
   Compiling num_cpus v1.17.0
   Compiling winapi-util v0.1.11
   Compiling same-file v1.0.6
   Compiling walkdir v2.5.0
   Compiling crossbeam-epoch v0.9.18
   Compiling crossbeam-deque v0.8.6
   Compiling syn v2.0.118
   Compiling block-buffer v0.10.4
   Compiling crypto-common v0.1.7
   Compiling digest v0.10.7
   Compiling sha2 v0.10.9
   Compiling rayon v1.12.0
   Compiling fs2 v0.4.3
   Compiling thiserror-impl v2.0.18
   Compiling serde_derive v1.0.228
   Compiling which v7.0.3
   Compiling cudaforge v0.1.6
   Compiling candle-kernels v0.10.2 (candle-kernels)
warning: candle-kernels@0.10.2: Compiling 11 of 11 PTX kernels
error: failed to run custom build command for `candle-kernels v0.10.2 (candle-kernels)`

Caused by:
  process didn't exit successfully: `Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-kernels\debug\build\candle-kernels-934663c982fd823d\build-script-build` (exit code: 1)
  --- stdout
  cargo::rerun-if-changed=build.rs
  cargo::rerun-if-changed=src/compatibility.cuh
  cargo::rerun-if-changed=src/cuda_utils.cuh
  cargo::rerun-if-changed=src/binary_op_macros.cuh
  cargo:rustc-env=CUDA_INCLUDE_DIR=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v13.3\include
  cargo:rerun-if-changed=src\affine.cu
  cargo:rerun-if-changed=src\binary.cu
  cargo:rerun-if-changed=src\cast.cu
  cargo:rerun-if-changed=src\conv.cu
  cargo:rerun-if-changed=src\fill.cu
  cargo:rerun-if-changed=src\indexing.cu
  cargo:rerun-if-changed=src\quantized.cu
  cargo:rerun-if-changed=src\reduce.cu
  cargo:rerun-if-changed=src\sort.cu
  cargo:rerun-if-changed=src\ternary.cu
  cargo:rerun-if-changed=src\unary.cu
  cargo:rerun-if-env-changed=CUDA_COMPUTE_CAP
  cargo:rerun-if-env-changed=NVCC_CCBIN
  cargo:warning=Compiling 11 of 11 PTX kernels
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH
  nvcc fatal   : Cannot find compiler 'cl.exe' in PATH

  --- stderr
  Error: CompilationFailed { path: "src\\affine.cu", message: "nvcc error:\n\n" } stdout={"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#windows-link@0.2.1","manifest_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\windows-link-0.2.1\\Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"windows_link","src_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\wi
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-kernels
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-kernels cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-kernels available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-nn cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-nn concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=300
[OK] Sandbox: processo efemero concluido command=cargo pid=18300 exit_code=0 stdout_bytes=365731 stderr_bytes=8306 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-examples
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-examples
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-examples cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-examples available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-pyo3 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-pyo3 concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=10256 exit_code=101 stdout_bytes=147672 stderr_bytes=4945 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-onnx
[ERR] Sidecar terminou com exit code nao zero binary=cargo exit_code=101 stderr=Updating crates.io index
     Locking 166 packages to latest compatible versions
   Compiling proc-macro2 v1.0.106
   Compiling unicode-ident v1.0.24
   Compiling quote v1.0.46
    Checking cfg-if v1.0.4
   Compiling getrandom v0.3.4
   Compiling zerocopy v0.8.52
   Compiling crossbeam-utils v0.8.21
   Compiling autocfg v1.5.1
   Compiling libm v0.2.16
   Compiling version_check v0.9.5
   Compiling paste v1.0.15
   Compiling serde_core v1.0.228
    Checking once_cell v1.21.4
   Compiling rayon-core v1.13.0
    Checking either v1.16.0
    Checking bitflags v2.13.0
    Checking memchr v2.8.2
    Checking reborrow v0.5.5
   Compiling serde v1.0.228
    Checking pulp-wasm-simd-flag v0.1.1
    Checking raw-cpuid v11.6.0
   Compiling dyn-stack-macros v0.1.3
   Compiling pulp v0.22.3
   Compiling strsim v0.11.1
   Compiling num-traits v0.2.19
   Compiling anyhow v1.0.102
   Compiling seq-macro v0.3.6
   Compiling ident_case v1.0.1
   Compiling fnv v1.0.7
   Compiling shlex v2.0.1
    Checking rand_core v0.9.5
   Compiling find-msvc-tools v0.1.9
   Compiling pkg-config v0.3.33
   Compiling itertools v0.14.0
   Compiling rustversion v1.0.22
   Compiling zmij v1.0.21
    Checking crossbeam-epoch v0.9.18
    Checking equivalent v1.0.2
    Checking itoa v1.0.18
   Compiling getrandom v0.4.3
   Compiling serde_json v1.0.150
   Compiling libc v0.2.186
    Checking aho-corasick v1.1.4
   Compiling ahash v0.8.12
   Compiling thiserror v2.0.18
   Compiling syn v2.0.118
    Checking crossbeam-deque v0.8.6
   Compiling cc v1.2.65
   Compiling crc32fast v1.5.0
   Compiling prettyplease v0.2.37
   Compiling hashbrown v0.17.1
   Compiling windows-link v0.2.1
   Compiling bytes v1.12.0
    Checking minimal-lexical v0.2.1
   Compiling esaxx-rs v0.1.10
    Checking regex-syntax v0.8.11
   Compiling foldhash v0.1.5
    Checking rayon v1.12.0
    Checking nom v7.1.3
    Checking windows-sys v0.61.2
   Compiling hashbrown v0.15.5
   Compiling indexmap v2.14.0
    Checking smallvec v1.15.2
    Checking static_assertions v1.1.0
   Compiling fixedbitset v0.5.7
    Checking fastrand v2.4.1
   Compiling macro_rules_attribute-proc_macro v0.2.2
    Checking regex-automata v0.4.14
    Checking stable_deref_trait v1.2.1
    Checking foldhash v0.2.0
    Checking ryu v1.0.23
    Checking base64 v0.13.1
   Compiling onig_sys v69.9.3
    Checking allocator-api2 v0.2.21
    Checking unicode-segmentation v1.13.3
    Checking tempfile v3.27.0
    Checking macro_rules_attribute v0.2.2
   Compiling petgraph v0.8.3
    Checking unicode-normalization-alignments v0.1.12
    Checking rayon-cond v0.4.0
    Checking hashbrown v0.16.1
    Checking log v0.4.33
   Compiling heck v0.5.0
    Checking typed-path v0.12.3
   Compiling multimap v0.10.1
    Checking unicode_categories v0.1.1
    Checking memmap2 v0.9.11
    Checking num_cpus v1.17.0
    Checking regex v1.12.4
    Checking byteorder v1.5.0
    Checking castaway v0.2.4
    Checking zip v8.6.0
   Compiling darling_core v0.20.11
   Compiling synstructure v0.13.2
   Compiling zerocopy-derive v0.8.52
   Compiling bytemuck_derive v1.10.2
   Compiling serde_derive v1.0.228
   Compiling prost-derive v0.14.4
   Compiling zerofrom-derive v0.1.7
   Compiling thiserror-impl v2.0.18
   Compiling monostate-impl v0.1.18
   Compiling yoke-derive v0.8.2
    Checking monostate v0.1.18
    Checking zerofrom v0.1.8
    Checking yoke v0.8.3
    Checking bytemuck v1.25.0
    Checking num-complex v0.4.6
    Checking dyn-stack v0.13.2
   Compiling darling_macro v0.20.11
   Compiling prost v0.14.4
   Compiling darling v0.20.11
   Compiling derive_builder_core v0.20.2
   Compiling prost-types v0.14.4
   Compiling derive_builder_macro v0.20.2
    Checking spm_precompiled v0.1.4
    Checking dary_heap v0.3.9
    Checking compact_str v0.9.1
    Checking safetensors v0.8.0
    Checking derive_builder v0.20.2
    Checking onig v6.5.3
   Compiling prost-build v0.14.4
   Compiling candle-onnx v0.10.2 (candle-onnx)
error: failed to run custom build command for `candle-onnx v0.10.2 (candle-onnx)`

Caused by:
  process didn't exit successfully: `Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-onnx\debug\build\candle-onnx-9dcd4e5f481b1b55\build-script-build` (exit code: 1)
  --- stderr
  Error: Custom { kind: NotFound, error: "Could not find `protoc`. If `protoc` is installed, try setting the `PROTOC` environment variable to the path of the `protoc` binary. Try installing `protobuf-compiler` or `protobuf` using your package manager. It is also available at https://github.com/protocolbuffers/protobuf/releases  For more information: https://docs.rs/prost-build/#sourcing-protoc" }
warning: build failed, waiting for other jobs to finish... stdout={"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.4","manifest_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"cfg_if","src_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\src\\l
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-onnx
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-onnx cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-onnx available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-transformers cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=330
[OK] Sandbox: processo efemero concluido command=cargo pid=6848 exit_code=101 stdout_bytes=80924 stderr_bytes=3038 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-pyo3
[ERR] Sidecar terminou com exit code nao zero binary=cargo exit_code=101 stderr=Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.46
   Compiling unicode-ident v1.0.24
    Checking cfg-if v1.0.4
   Compiling autocfg v1.5.1
   Compiling getrandom v0.3.4
   Compiling zerocopy v0.8.52
   Compiling crossbeam-utils v0.8.21
   Compiling target-lexicon v0.13.5
   Compiling libm v0.2.16
   Compiling version_check v0.9.5
    Checking once_cell v1.21.4
   Compiling serde_core v1.0.228
   Compiling paste v1.0.15
   Compiling rayon-core v1.13.0
    Checking bitflags v2.13.0
    Checking either v1.16.0
    Checking memchr v2.8.2
    Checking raw-cpuid v11.6.0
   Compiling serde v1.0.228
    Checking pulp-wasm-simd-flag v0.1.1
   Compiling pulp v0.22.3
   Compiling dyn-stack-macros v0.1.3
   Compiling num-traits v0.2.19
    Checking reborrow v0.5.5
   Compiling strsim v0.11.1
   Compiling fnv v1.0.7
   Compiling ident_case v1.0.1
   Compiling seq-macro v0.3.6
   Compiling shlex v2.0.1
   Compiling find-msvc-tools v0.1.9
   Compiling libc v0.2.186
    Checking rand_core v0.9.5
   Compiling pkg-config v0.3.33
   Compiling zmij v1.0.21
   Compiling rustversion v1.0.22
   Compiling getrandom v0.4.3
    Checking equivalent v1.0.2
   Compiling serde_json v1.0.150
    Checking crossbeam-epoch v0.9.18
    Checking itoa v1.0.18
   Compiling pyo3-build-config v0.27.2
   Compiling cc v1.2.65
    Checking aho-corasick v1.1.4
   Compiling ahash v0.8.12
    Checking crossbeam-deque v0.8.6
   Compiling syn v2.0.118
   Compiling crc32fast v1.5.0
    Checking minimal-lexical v0.2.1
    Checking regex-syntax v0.8.11
   Compiling thiserror v2.0.18
    Checking windows-link v0.2.1
   Compiling esaxx-rs v0.1.10
    Checking windows-sys v0.61.2
    Checking nom v7.1.3
    Checking itertools v0.14.0
    Checking smallvec v1.15.2
    Checking static_assertions v1.1.0
    Checking rayon v1.12.0
    Checking base64 v0.13.1
    Checking foldhash v0.2.0
    Checking allocator-api2 v0.2.21
    Checking fastrand v2.4.1
    Checking ryu v1.0.23
    Checking stable_deref_trait v1.2.1
    Checking hashbrown v0.17.1
    Checking unicode-segmentation v1.13.3
   Compiling macro_rules_attribute-proc_macro v0.2.2
   Compiling onig_sys v69.9.3
    Checking hashbrown v0.16.1
    Checking regex-automata v0.4.14
error: failed to run custom build command for `pyo3-build-config v0.27.2`

Caused by:
  process didn't exit successfully: `Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-pyo3\debug\build\pyo3-build-config-ab9f0a0d7898bb73\build-script-build` (exit code: 1)
  --- stdout
  cargo:rerun-if-env-changed=PYO3_CONFIG_FILE
  cargo:rerun-if-env-changed=PYO3_NO_PYTHON
  cargo:rerun-if-env-changed=PYO3_ENVIRONMENT_SIGNATURE
  cargo:rerun-if-env-changed=PYO3_PYTHON
  cargo:rerun-if-env-changed=VIRTUAL_ENV
  cargo:rerun-if-env-changed=CONDA_PREFIX
  cargo:rerun-if-env-changed=PATH

  --- stderr
  error: cannot set a minimum Python version 3.13 higher than the interpreter version 3.12 (the minimum Python version is implied by the abi3-py313 feature)
warning: build failed, waiting for other jobs to finish... stdout={"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#cfg-if@1.0.4","manifest_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\Cargo.toml","target":{"kind":["lib"],"crate_types":["lib"],"name":"cfg_if","src_path":"C:\\Users\\rosas\\.cargo\\registry\\src\\index.crates.io-1949cf8c6b5b557f\\cfg-if-1.0.4\\src\\l
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-pyo3
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-pyo3 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-pyo3 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/bert cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\bert concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=360
[OK] Sandbox: processo efemero concluido command=cargo pid=21176 exit_code=0 stdout_bytes=131627 stderr_bytes=4257 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-nn
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-nn
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-nn cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-nn available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-ug cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-ug concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=390
[OK] Sandbox: processo efemero concluido command=cargo pid=11632 exit_code=0 stdout_bytes=80463 stderr_bytes=2495 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-ug
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-ug
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-ug cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-ug available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/blip cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\blip concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=420
[OK] Sandbox: processo efemero concluido command=cargo pid=14728 exit_code=0 stdout_bytes=141915 stderr_bytes=4703 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-transformers
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-transformers cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/chat-template cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\chat-template concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=26612 exit_code=0 stdout_bytes=192438 stderr_bytes=6602 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\bert
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\bert
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/bert cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\bert available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/llama2-c cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\llama2-c concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=450
[OK] Sandbox: processo efemero concluido command=cargo pid=17608 exit_code=0 stdout_bytes=32602 stderr_bytes=954 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\chat-template
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\chat-template
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/chat-template cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\chat-template available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/moondream cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\moondream concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=480
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=510
[OK] Sandbox: processo efemero concluido command=cargo pid=19304 exit_code=0 stdout_bytes=166419 stderr_bytes=5738 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\blip
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\blip
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/blip cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\blip available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/phi cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\phi concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=540
[OK] Sandbox: processo efemero concluido command=cargo pid=476 exit_code=0 stdout_bytes=248960 stderr_bytes=8256 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\llama2-c
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\llama2-c
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/llama2-c cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\llama2-c available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/quant-qwen3 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\quant-qwen3 concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=570
[OK] Sandbox: processo efemero concluido command=cargo pid=32596 exit_code=0 stdout_bytes=168801 stderr_bytes=5787 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\moondream
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\moondream
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/moondream cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\moondream available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/segment-anything cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\segment-anything concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=600
[OK] Sandbox: processo efemero concluido command=cargo pid=9892 exit_code=0 stdout_bytes=166102 stderr_bytes=5736 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\phi
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\phi
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/phi cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\phi available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/t5 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\t5 concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=630
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=660
[OK] Sandbox: processo efemero concluido command=cargo pid=45364 exit_code=0 stdout_bytes=170422 stderr_bytes=6014 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\quant-qwen3
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\quant-qwen3
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/quant-qwen3 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\quant-qwen3 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/whisper cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\whisper concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=39756 exit_code=0 stdout_bytes=170121 stderr_bytes=5755 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\segment-anything
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\segment-anything
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/segment-anything cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\segment-anything available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-examples/yolo cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\yolo concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=690
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=720
[OK] Sandbox: processo efemero concluido command=cargo pid=33344 exit_code=0 stdout_bytes=192717 stderr_bytes=6598 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\t5
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\t5
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/t5 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\t5 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=candle-wasm-tests cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-tests concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=750
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=780
[OK] Sandbox: processo efemero concluido command=cargo pid=31264 exit_code=0 stdout_bytes=134449 stderr_bytes=4304 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-tests
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\candle-wasm-tests
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-tests cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-tests available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=rust-clippy scope=tensor-tools cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\tensor-tools concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=25212 exit_code=0 stdout_bytes=251571 stderr_bytes=8262 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\yolo
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\yolo
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/yolo cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\yolo available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=cppcheck scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cppcheck pid=34352 exit_code=1 stdout_bytes=533 stderr_bytes=7577 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[PROC] SAST monorepo: sub-scan concluído blade=cppcheck scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=. cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=13316 exit_code=1 stdout_bytes=76446 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=. cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=. cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=13732 exit_code=0 stdout_bytes=5917 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=. cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=ruff scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=810
[OK] Sandbox: processo efemero concluido command=ruff pid=40644 exit_code=1 stdout_bytes=95386 stderr_bytes=3428 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[PROC] SAST monorepo: sub-scan concluído blade=ruff scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=bandit scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=bandit pid=42688 exit_code=1 stdout_bytes=133967 stderr_bytes=314 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle
[PROC] SAST monorepo: sub-scan concluído blade=bandit scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle available_permits=0
[OK] SAST monorepo: permissão adquirida blade=opengrep scope=candle-book/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-book\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cargo pid=21436 exit_code=0 stdout_bytes=249229 stderr_bytes=8282 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\whisper
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\whisper
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=candle-wasm-examples/whisper cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-wasm-examples\whisper available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-core/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-core\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=795 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=cargo pid=12256 exit_code=0 stdout_bytes=146582 stderr_bytes=4742 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\tensor-tools
[PROC] clippy: cache efemero removido target_dir=Z:\genesis_mc\.soda_sandbox\cargo-clippy-target\tensor-tools
[PROC] SAST monorepo: sub-scan concluído blade=rust-clippy scope=tensor-tools cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\tensor-tools available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-examples/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-examples\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=840
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=870
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=900
[OK] Sandbox: processo efemero concluido command=opengrep pid=25492 exit_code=0 stdout_bytes=71627 stderr_bytes=1277 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-examples\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-examples/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-examples\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-flash-attn-v3/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn-v3\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=29680 exit_code=0 stdout_bytes=49526 stderr_bytes=1329 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-book\src
[OK] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-book/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-book\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-flash-attn/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=930
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=960
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=990
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1020
[OK] Sandbox: processo efemero concluido command=opengrep pid=18724 exit_code=0 stdout_bytes=87236 stderr_bytes=1877 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn-v3\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-flash-attn-v3/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn-v3\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-kernels/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-kernels\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=2960 exit_code=0 stdout_bytes=75163 stderr_bytes=1777 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-flash-attn/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-flash-attn\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-datasets/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-datasets\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1050
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1080
[ERR] Sandbox: idle timeout atingido; aniquilando sidecar command=opengrep pid=35660 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-core\src idle_timeout_secs=180 absolute_timeout_secs=600
[ERR] Sandbox: sidecar aniquilado apos timeout command=opengrep pid=35660 stdout_bytes=0 stderr_bytes=1886 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-core\src timeout_kind=idle
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-core/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-core\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-metal-kernels/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-metal-kernels\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1110
[OK] Sandbox: processo efemero concluido command=opengrep pid=22596 exit_code=0 stdout_bytes=35560 stderr_bytes=1227 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-datasets\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-datasets/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-datasets\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-nn/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-nn\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1140
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1170
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1200
[OK] Sandbox: processo efemero concluido command=opengrep pid=18756 exit_code=0 stdout_bytes=1020611 stderr_bytes=1330 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-kernels\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-kernels/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-kernels\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-onnx/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-onnx\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1230
[OK] Sandbox: processo efemero concluido command=opengrep pid=26160 exit_code=0 stdout_bytes=589014 stderr_bytes=1380 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-nn\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-nn/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-nn\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-pyo3/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-pyo3\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1260
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1290
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1320
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1350
[OK] Sandbox: processo efemero concluido command=opengrep pid=2028 exit_code=0 stdout_bytes=1235632 stderr_bytes=1430 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-metal-kernels\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-metal-kernels/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-metal-kernels\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/generation cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\generation concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=2472 exit_code=0 stdout_bytes=386744 stderr_bytes=1228 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-onnx\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-onnx/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-onnx\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/chinese_clip cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\chinese_clip concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=23368 exit_code=0 stdout_bytes=122558 stderr_bytes=1227 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-pyo3\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-pyo3/src cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-pyo3\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/clip cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\clip concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1380
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1410
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1440
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1470
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1500
[OK] Sandbox: processo efemero concluido command=opengrep pid=22844 exit_code=0 stdout_bytes=6465 stderr_bytes=2224 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\generation
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/generation cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\generation available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/flux cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\flux concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=30576 exit_code=0 stdout_bytes=6381 stderr_bytes=2726 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\clip
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/clip cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\clip available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/gemma4 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\gemma4 concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=33420 exit_code=0 stdout_bytes=14791 stderr_bytes=2027 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\chinese_clip
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/chinese_clip cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\chinese_clip available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/llava cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\llava concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1530
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1560
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1590
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1620
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1650
[OK] Sandbox: processo efemero concluido command=opengrep pid=44780 exit_code=0 stdout_bytes=30149 stderr_bytes=2226 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\flux
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/flux cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\flux available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/mimi cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\mimi concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=28060 exit_code=0 stdout_bytes=18753 stderr_bytes=2476 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\llava
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/llava cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\llava available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/mmdit cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\mmdit concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=23244 exit_code=0 stdout_bytes=86010 stderr_bytes=2877 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\gemma4
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/gemma4 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\gemma4 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/nvembed_v2 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\nvembed_v2 concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1680
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1710
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1740
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1770
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1800
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1830
[OK] Sandbox: processo efemero concluido command=opengrep pid=44276 exit_code=0 stdout_bytes=1756 stderr_bytes=2475 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\mmdit
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/mmdit cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\mmdit available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/openclip cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\openclip concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=41784 exit_code=0 stdout_bytes=11479 stderr_bytes=2176 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\nvembed_v2
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/nvembed_v2 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\nvembed_v2 available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models/paddleocr_vl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\paddleocr_vl concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=43328 exit_code=0 stdout_bytes=163729 stderr_bytes=2127 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\mimi
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/mimi cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\mimi available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1860
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1890
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1920
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1950
[OK] Sandbox: processo efemero concluido command=opengrep pid=36520 exit_code=0 stdout_bytes=1030 stderr_bytes=1225 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\openclip
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/openclip cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\openclip available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models::files-02 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=24516 exit_code=0 stdout_bytes=200816 stderr_bytes=1227 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\paddleocr_vl
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models/paddleocr_vl cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models\paddleocr_vl available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models::files-04 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=1980
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2010
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2040
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2070
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2100
[OK] Sandbox: processo efemero concluido command=opengrep pid=22688 exit_code=0 stdout_bytes=492184 stderr_bytes=1280 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models::files-05 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=13076 exit_code=0 stdout_bytes=423157 stderr_bytes=1230 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models::files-02 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2130
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2160
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2190
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2220
[OK] Sandbox: processo efemero concluido command=opengrep pid=2404 exit_code=0 stdout_bytes=472024 stderr_bytes=1230 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models::files-04 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=candle-transformers/src/models::files-03 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\.soda_semgrep\candle\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=6376 exit_code=0 stdout_bytes=18103 stderr_bytes=1227 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src available_permits=1
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2250
[OK] Sandbox: processo efemero concluido command=opengrep pid=40524 exit_code=0 stdout_bytes=182374 stderr_bytes=1279 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models::files-05 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models available_permits=2
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2280
[PROC] F0: heartbeat repo_id=huggingface/candle elapsed_s=2310
[OK] Sandbox: processo efemero concluido command=opengrep pid=31060 exit_code=0 stdout_bytes=242493 stderr_bytes=1229 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=candle-transformers/src/models::files-03 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle\candle-transformers\src\models available_permits=3
[FINOPS] N11: roteador poliglota de SAST concluido repo_id=huggingface/candle elapsed_ms=2287795 unsafe_hotspots_bytes=24962 health_report_bytes=259286
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_06_unsafe_hotspots payload_bytes=24962
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_08_health_report payload_bytes=259286
[PROC] N10: Finalizando coleta de metadados comunitarios repo_id=huggingface/candle
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_09_community_meta payload_bytes=2012
[PROC] N11: Extraindo blob_10_soda_canon_context repo_id=huggingface/candle
[PROC] blob_10_soda_canon_context servido do cache SQLite repo_id=huggingface/candle
[PROC] Blob gerado repo_id=huggingface/candle artifact_type=blob_10_soda_canon_context payload_bytes=4648
[PROC] N12: Persistindo pacote RAW no SQLite repo_id=huggingface/candle blobs_count=11 total_payload_bytes=652397
[OK] N12: Persistencia do pacote RAW concluida repo_id=huggingface/candle
[OK] N13: pipeline_core retornou; iniciando teardown repo_id=huggingface/candle is_ok=true
[PROC] N13: PurgeGuard iniciando limpeza atomica (Sandbox + TempWorkspace) repo_id=huggingface/candle
[PROC] PurgeGuard: Iniciando limpeza atômica de recursos
[PROC] PurgeGuard: SandboxHandle descartado
[PROC] RamdiskHandle: iniciando teardown ProjFS path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900 projected_roots=1
[FINOPS] RamdiskHandle: virtualization root delegada para delecao externa path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900\repos\huggingface\candle elapsed_ms=16
[FINOPS] RamdiskHandle: cleanup explicito concluido com delecao externa não-bloqueante path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782281264489518900 elapsed_ms=32
[PROC] RamdiskGuard: cleanup ja delegado externamente; Drop nao repetira a remocao
[PROC] PurgeGuard: RamdiskHandle descartado
[PROC] N13: Teardown finalizado; retornando ao CLI repo_id=huggingface/candle
[FINOPS] F0: concluído repo_id=huggingface/candle row_number=365 report=Z:\genesis_mc\.soda_scratchpad\reports\_ETL_REPORT_huggingface_candle.txt elapsed_ms=2319631
[PROC] F0(batch): iniciando repo_id=agentjido/jido row_number=394 idx=3 total=5
[PROC] Iniciando HarvesterOrchestrator (N14) url=https://github.com/agentjido/jido repo_id=agentjido/jido
[PROC] N1: Alocando workspace efemero da F0 repo_id=agentjido/jido requested_mb=256
[PROC] N1: Workspace efemero pronto repo_id=agentjido/jido workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200
[PROC] N2: Iniciando clone blobless repo_id=agentjido/jido url=https://github.com/agentjido/jido
[PROC] Preparando workspace efemero do clone url=https://github.com/agentjido/jido workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200 dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200\repos\agentjido\jido
[PROC] ProjFS: consultando metadados do repositório GitHub url=https://api.github.com/repos/agentjido/jido
[PROC] ProjFS: consultando release mais recente do repositório url=https://api.github.com/repos/agentjido/jido/releases/latest
[PROC] ProjFS: consultando SHA do commit HEAD url=https://api.github.com/repos/agentjido/jido/commits?sha=main&per_page=1
[PROC] ProjFS: baixando snapshot compactado do repositório url=https://api.github.com/repos/agentjido/jido/zipball/main default_branch=main selected_branch=main
[PROC] ProjFS: snapshot ZIP recebido em memória archive_bytes=789657
[FINOPS] Clone virtual via ProjFS concluido url=https://github.com/agentjido/jido dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200\repos\agentjido\jido projected_files=368 projected_bytes=3020900 elapsed_ms=1838
[OK] N2: Clone blobless concluido repo_id=agentjido/jido repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200\repos\agentjido\jido
[PROC] N3: Criando sandbox efemero repo_id=agentjido/jido repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200\repos\agentjido\jido
[PROC] N3: Sandbox pronto repo_id=agentjido/jido
[PROC] N4: Detectando stack do repositório repo_id=agentjido/jido
[PROC] N4: Stack detectada repo_id=agentjido/jido profile=Elixir
[PROC] N5: Roteando tarefas de extração repo_id=agentjido/jido
[PROC] N5: Tarefas roteadas repo_id=agentjido/jido tasks=[RunNativeAstParser, ExtractManifests, RunStaticAnalysis, FetchCommunityMeta, ExtractOpsBlueprint]
[PROC] N10: Iniciando coleta concorrente de metadados comunitarios repo_id=agentjido/jido
[PROC] ast-native: iniciando extração estrutural repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200\repos\agentjido\jido
[ERR] Falha critica ao extrair blob_04_repo_outline repo_id=agentjido/jido error=Execution failed: Nenhum símbolo estrutural foi extraído do repositório '\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200\repos\agentjido\jido'
[OK] N13: pipeline_core retornou; iniciando teardown repo_id=agentjido/jido is_ok=false
[PROC] N13: PurgeGuard iniciando limpeza atomica (Sandbox + TempWorkspace) repo_id=agentjido/jido
[PROC] PurgeGuard: Iniciando limpeza atômica de recursos
[PROC] PurgeGuard: SandboxHandle descartado
[PROC] RamdiskHandle: iniciando teardown ProjFS path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200 projected_roots=1
[FINOPS] RamdiskHandle: virtualization root delegada para delecao externa path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200\repos\agentjido\jido elapsed_ms=16
[FINOPS] RamdiskHandle: cleanup explicito concluido com delecao externa não-bloqueante path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200 elapsed_ms=28
[PROC] RamdiskGuard: cleanup ja delegado externamente; Drop nao repetira a remocao
[PROC] PurgeGuard: RamdiskHandle descartado
[PROC] N13: Teardown finalizado; retornando ao CLI repo_id=agentjido/jido
[ERR] F0: falha fatal (fail-soft por repo) repo_id=agentjido/jido row_number=394 error=Extraction failed: Execution failed: Nenhum símbolo estrutural foi extraído do repositório '\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283587387901200\repos\agentjido\jido'
[PROC] F0(batch): iniciando repo_id=multigres/multigres-operator row_number=562 idx=4 total=5
[PROC] Iniciando HarvesterOrchestrator (N14) url=https://github.com/multigres/multigres-operator repo_id=multigres/multigres-operator
[PROC] N1: Alocando workspace efemero da F0 repo_id=multigres/multigres-operator requested_mb=256
[PROC] N1: Workspace efemero pronto repo_id=multigres/multigres-operator workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000
[PROC] N2: Iniciando clone blobless repo_id=multigres/multigres-operator url=https://github.com/multigres/multigres-operator
[PROC] Preparando workspace efemero do clone url=https://github.com/multigres/multigres-operator workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000 dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator
[PROC] ProjFS: consultando metadados do repositório GitHub url=https://api.github.com/repos/multigres/multigres-operator
[PROC] ProjFS: consultando release mais recente do repositório url=https://api.github.com/repos/multigres/multigres-operator/releases/latest
[PROC] ProjFS: consultando SHA do commit HEAD url=https://api.github.com/repos/multigres/multigres-operator/commits?sha=main&per_page=1
[PROC] ProjFS: baixando snapshot compactado do repositório url=https://api.github.com/repos/multigres/multigres-operator/zipball/main default_branch=main selected_branch=main
[PROC] ProjFS: snapshot ZIP recebido em memória archive_bytes=1589732
[FINOPS] Clone virtual via ProjFS concluido url=https://github.com/multigres/multigres-operator dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator projected_files=516 projected_bytes=6823834 elapsed_ms=3128
[OK] N2: Clone blobless concluido repo_id=multigres/multigres-operator repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator
[PROC] N3: Criando sandbox efemero repo_id=multigres/multigres-operator repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator
[PROC] N3: Sandbox pronto repo_id=multigres/multigres-operator
[PROC] N4: Detectando stack do repositório repo_id=multigres/multigres-operator
[PROC] N4: Stack detectada repo_id=multigres/multigres-operator profile=Mixed([CCpp, NodeJS, Go, Python])
[PROC] N5: Roteando tarefas de extração repo_id=multigres/multigres-operator
[PROC] N5: Tarefas roteadas repo_id=multigres/multigres-operator tasks=[RunNativeAstParser, ExtractManifests, RunStaticAnalysis, FetchCommunityMeta, ExtractOpsBlueprint, RunOxc, DiscoverTests]
[PROC] N10: Iniciando coleta concorrente de metadados comunitarios repo_id=multigres/multigres-operator
[PROC] ast-native: iniciando extração estrutural repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=30
[PROC] ast-native: artefatos normalizados repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator repo_outline_bytes=83753 architecture_map_bytes=12154 health_report_bytes=222
[FINOPS] N6: parser AST nativo concluido repo_id=multigres/multigres-operator elapsed_ms=42161 repo_outline_bytes=83753 architecture_map_bytes=12154
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_04_repo_outline payload_bytes=83753
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_05_architecture_map payload_bytes=12154
[PROC] N7: Extraindo blob_01_promessa_readme repo_id=multigres/multigres-operator
[PROC] Tentando ler arquivo para artefato artifact_type=blob_01_promessa_readme candidate=README.md abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\README.md
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_01_promessa_readme payload_bytes=18195
[PROC] N8: Extraindo blob_02_dependency_manifest repo_id=multigres/multigres-operator
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=go.mod abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\go.mod
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=tools/observer/go.mod abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\observer\go.mod
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_02_dependency_manifest payload_bytes=6214
[PROC] N9: Extraindo blob_07_ops_blueprint repo_id=multigres/multigres-operator
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_07_ops_blueprint payload_bytes=88215
[PROC] N11: Extraindo blob_03_test_intent repo_id=multigres/multigres-operator
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_03_test_intent payload_bytes=47460
[PROC] N11: Extraindo blob_11_ux_contracts repo_id=multigres/multigres-operator
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_11_ux_contracts payload_bytes=30
[PROC] SAST monorepo: manifestos detectados repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator manifest_count=2 manifests=[".:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782283606818954000\\repos\\multigres\\multigres-operator\\go.mod", "tools/observer:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782283606818954000\\repos\\multigres\\multigres-operator\\tools\\observer\\go.mod"] concurrency_limit=3
[PROC] SAST monorepo: permissão adquirida blade=cppcheck scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator concurrency_limit=3 in_flight=1
[PROC] SAST monorepo: permissão adquirida blade=biome scope=. cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator concurrency_limit=3 in_flight=2
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=. cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=cppcheck pid=11984 exit_code=1 stdout_bytes=65 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator
[PROC] SAST monorepo: sub-scan concluído blade=cppcheck scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=govulncheck scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=22564 exit_code=0 stdout_bytes=1067 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=. cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=ruff scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=ruff pid=13612 exit_code=0 stdout_bytes=2 stderr_bytes=238 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator
[PROC] SAST monorepo: sub-scan concluído blade=ruff scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=bandit scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=26180 exit_code=1 stdout_bytes=3023 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=. cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=govulncheck scope=tools/observer cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\observer concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=bandit pid=22396 exit_code=1 stdout_bytes=3523 stderr_bytes=156 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator
[PROC] SAST monorepo: sub-scan concluído blade=bandit scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=cmd cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\cmd concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=60
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator rule_set=Health copied_rule_files=795 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\.soda_semgrep\multigres-operator\health
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=90
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=120
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=150
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=180
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=210
[ERR] Sandbox: idle timeout atingido; aniquilando sidecar command=govulncheck pid=36076 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator idle_timeout_secs=180 absolute_timeout_secs=600
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=240
[ERR] Sandbox: sidecar aniquilado apos timeout command=govulncheck pid=36076 stdout_bytes=289 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator timeout_kind=idle
[PROC] SAST monorepo: sub-scan concluído blade=govulncheck scope=. cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\scripts concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\.soda_semgrep\multigres-operator\health
[ERR] Sandbox: idle timeout atingido; aniquilando sidecar command=opengrep pid=32984 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\cmd idle_timeout_secs=180 absolute_timeout_secs=600
[ERR] Sandbox: sidecar aniquilado apos timeout command=opengrep pid=32984 stdout_bytes=0 stderr_bytes=708 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\cmd timeout_kind=idle
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=cmd cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\cmd available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=tools/observer/cmd cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\observer\cmd concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\.soda_semgrep\multigres-operator\health
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=270
[OK] Sandbox: processo efemero concluido command=govulncheck pid=11000 exit_code=0 stdout_bytes=538790 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\observer
[PROC] SAST monorepo: sub-scan concluído blade=govulncheck scope=tools/observer cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\observer available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=tools/skills/pin_upstream_images/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\skills\pin_upstream_images\scripts concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\.soda_semgrep\multigres-operator\health
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=300
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=330
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=360
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=390
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=420
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=450
[OK] Sandbox: processo efemero concluido command=opengrep pid=36744 exit_code=0 stdout_bytes=2781 stderr_bytes=3973 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\observer\cmd
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=tools/observer/cmd cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\observer\cmd available_permits=1
[OK] Sandbox: processo efemero concluido command=opengrep pid=42440 exit_code=0 stdout_bytes=6799 stderr_bytes=4124 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\skills\pin_upstream_images\scripts
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=tools/skills/pin_upstream_images/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\tools\skills\pin_upstream_images\scripts available_permits=2
[OK] Sandbox: processo efemero concluido command=opengrep pid=32012 exit_code=0 stdout_bytes=47477 stderr_bytes=4125 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\scripts
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator\scripts available_permits=3
[FINOPS] N11: roteador poliglota de SAST concluido repo_id=multigres/multigres-operator elapsed_ms=422702 unsafe_hotspots_bytes=698 health_report_bytes=4839
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_06_unsafe_hotspots payload_bytes=698
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_08_health_report payload_bytes=4839
[PROC] N10: Finalizando coleta de metadados comunitarios repo_id=multigres/multigres-operator
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_09_community_meta payload_bytes=1938
[PROC] N11: Extraindo blob_10_soda_canon_context repo_id=multigres/multigres-operator
[PROC] blob_10_soda_canon_context servido do cache SQLite repo_id=multigres/multigres-operator
[PROC] Blob gerado repo_id=multigres/multigres-operator artifact_type=blob_10_soda_canon_context payload_bytes=4648
[PROC] N12: Persistindo pacote RAW no SQLite repo_id=multigres/multigres-operator blobs_count=11 total_payload_bytes=268144
[OK] N12: Persistencia do pacote RAW concluida repo_id=multigres/multigres-operator
[OK] N13: pipeline_core retornou; iniciando teardown repo_id=multigres/multigres-operator is_ok=true
[PROC] N13: PurgeGuard iniciando limpeza atomica (Sandbox + TempWorkspace) repo_id=multigres/multigres-operator
[PROC] PurgeGuard: Iniciando limpeza atômica de recursos
[PROC] PurgeGuard: SandboxHandle descartado
[PROC] RamdiskHandle: iniciando teardown ProjFS path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000 projected_roots=1
[FINOPS] RamdiskHandle: virtualization root delegada para delecao externa path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000\repos\multigres\multigres-operator elapsed_ms=48
[FINOPS] RamdiskHandle: cleanup explicito concluido com delecao externa não-bloqueante path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782283606818954000 elapsed_ms=98
[PROC] RamdiskGuard: cleanup ja delegado externamente; Drop nao repetira a remocao
[PROC] F0: heartbeat repo_id=multigres/multigres-operator elapsed_s=480
[PROC] PurgeGuard: RamdiskHandle descartado
[PROC] N13: Teardown finalizado; retornando ao CLI repo_id=multigres/multigres-operator
[FINOPS] F0: concluído repo_id=multigres/multigres-operator row_number=562 report=Z:\genesis_mc\.soda_scratchpad\reports\_ETL_REPORT_multigres_multigres-operator.txt elapsed_ms=488417
[PROC] F0(batch): iniciando repo_id=sveltejs/svelte row_number=768 idx=5 total=5
[PROC] Iniciando HarvesterOrchestrator (N14) url=https://github.com/sveltejs/svelte repo_id=sveltejs/svelte
[PROC] N1: Alocando workspace efemero da F0 repo_id=sveltejs/svelte requested_mb=256
[PROC] N1: Workspace efemero pronto repo_id=sveltejs/svelte workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500
[PROC] N2: Iniciando clone blobless repo_id=sveltejs/svelte url=https://github.com/sveltejs/svelte
[PROC] Preparando workspace efemero do clone url=https://github.com/sveltejs/svelte workspace=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500 dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte
[PROC] ProjFS: consultando metadados do repositório GitHub url=https://api.github.com/repos/sveltejs/svelte
[PROC] ProjFS: consultando release mais recente do repositório url=https://api.github.com/repos/sveltejs/svelte/releases/latest
[PROC] ProjFS: consultando SHA do commit HEAD url=https://api.github.com/repos/sveltejs/svelte/commits?sha=main&per_page=1
[PROC] ProjFS: baixando snapshot compactado do repositório url=https://api.github.com/repos/sveltejs/svelte/zipball/main default_branch=main selected_branch=main
[PROC] ProjFS: snapshot ZIP recebido em memória archive_bytes=6350873
[FINOPS] Clone virtual via ProjFS concluido url=https://github.com/sveltejs/svelte dest=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte projected_files=8944 projected_bytes=6961107 elapsed_ms=7271
[OK] N2: Clone blobless concluido repo_id=sveltejs/svelte repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte
[PROC] N3: Criando sandbox efemero repo_id=sveltejs/svelte repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte
[PROC] N3: Sandbox pronto repo_id=sveltejs/svelte
[PROC] N4: Detectando stack do repositório repo_id=sveltejs/svelte
[PROC] N4: Stack detectada repo_id=sveltejs/svelte profile=NodeJS
[PROC] N5: Roteando tarefas de extração repo_id=sveltejs/svelte
[PROC] N5: Tarefas roteadas repo_id=sveltejs/svelte tasks=[RunNativeAstParser, RunOxc, DiscoverTests, ExtractManifests, RunStaticAnalysis, FetchCommunityMeta, ExtractOpsBlueprint]
[PROC] N10: Iniciando coleta concorrente de metadados comunitarios repo_id=sveltejs/svelte
[PROC] ast-native: iniciando extração estrutural repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=30
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=60
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=90
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=120
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=150
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=180
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=210
[PROC] ast-native: artefatos normalizados repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte repo_outline_bytes=643969 architecture_map_bytes=28640 health_report_bytes=209
[FINOPS] N6: parser AST nativo concluido repo_id=sveltejs/svelte elapsed_ms=208148 repo_outline_bytes=643969 architecture_map_bytes=28640
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_04_repo_outline payload_bytes=643969
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_05_architecture_map payload_bytes=28640
[PROC] N7: Extraindo blob_01_promessa_readme repo_id=sveltejs/svelte
[PROC] Tentando ler arquivo para artefato artifact_type=blob_01_promessa_readme candidate=README.md abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\README.md
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_01_promessa_readme payload_bytes=1448
[PROC] N8: Extraindo blob_02_dependency_manifest repo_id=sveltejs/svelte
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=packages/svelte/compiler/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\compiler\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=packages/svelte/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\package.json
[PROC] Tentando ler manifesto artifact_type=blob_02_dependency_manifest manifest=playgrounds/sandbox/package.json abs_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\playgrounds\sandbox\package.json
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_02_dependency_manifest payload_bytes=1085
[PROC] N9: Extraindo blob_07_ops_blueprint repo_id=sveltejs/svelte
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_07_ops_blueprint payload_bytes=21117
[PROC] N11: Extraindo blob_03_test_intent repo_id=sveltejs/svelte
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_03_test_intent payload_bytes=5431
[PROC] N11: Extraindo blob_11_ux_contracts repo_id=sveltejs/svelte
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_11_ux_contracts payload_bytes=30
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=240
[PROC] SAST monorepo: manifestos detectados repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte manifest_count=4 manifests=[".:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782284101719926500\\repos\\sveltejs\\svelte\\package.json", "packages/svelte:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782284101719926500\\repos\\sveltejs\\svelte\\packages\\svelte\\package.json", "packages/svelte/compiler:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782284101719926500\\repos\\sveltejs\\svelte\\packages\\svelte\\compiler\\package.json", "playgrounds/sandbox:C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782284101719926500\\repos\\sveltejs\\svelte\\playgrounds\\sandbox\\package.json"] concurrency_limit=3
[PROC] SAST monorepo: permissão adquirida blade=biome scope=.::files-01 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte concurrency_limit=3 in_flight=1
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/compiler cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\compiler concurrency_limit=3 in_flight=2
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\scripts concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=5276 exit_code=1 stdout_bytes=441 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\compiler
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/compiler cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\compiler available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/animate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\animate concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=11044 exit_code=1 stdout_bytes=1324 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=.::files-01 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler/migrate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\migrate concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=38476 exit_code=1 stdout_bytes=10730 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\scripts
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\scripts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/attachments cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\attachments concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=19780 exit_code=1 stdout_bytes=647 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\animate
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/animate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\animate available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler/phases/1-parse cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\1-parse concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=14816 exit_code=1 stdout_bytes=1370 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\attachments
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/attachments cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\attachments available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler/phases/2-analyze cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\2-analyze concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=13688 exit_code=1 stdout_bytes=9947 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\migrate
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler/migrate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\migrate available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler/phases::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=23304 exit_code=1 stdout_bytes=4698 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler/phases::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler/phases/3-transform cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\3-transform concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=9812 exit_code=1 stdout_bytes=16715 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\1-parse
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler/phases/1-parse cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\1-parse available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler/preprocess cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\preprocess concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=41788 exit_code=1 stdout_bytes=37436 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\2-analyze
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler/phases/2-analyze cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\2-analyze available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler/print cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\print concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=21204 exit_code=1 stdout_bytes=6297 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\preprocess
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler/preprocess cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\preprocess available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler/types cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\types concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=22272 exit_code=1 stdout_bytes=2148 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\print
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler/print cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\print available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler/utils cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\utils concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=19136 exit_code=1 stdout_bytes=4460 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\types
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler/types cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\types available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/compiler::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=22028 exit_code=1 stdout_bytes=65227 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\3-transform
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler/phases/3-transform cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\3-transform available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/easing cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\easing concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=37020 exit_code=1 stdout_bytes=10978 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\utils
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler/utils cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\utils available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/events cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\events concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=3832 exit_code=1 stdout_bytes=4913 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\easing
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/easing cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\easing available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/internal/client/dom cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=10568 exit_code=1 stdout_bytes=1563 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\events
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/events cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\events available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/internal/client/reactivity cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\reactivity concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=11516 exit_code=1 stdout_bytes=10219 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/compiler::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/internal/client::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=11268 exit_code=1 stdout_bytes=27976 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\reactivity
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/internal/client/reactivity cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\reactivity available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/internal/flags cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\flags concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=44424 exit_code=1 stdout_bytes=80438 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/internal/client/dom cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/internal/server cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\server concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=26808 exit_code=1 stdout_bytes=26139 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/internal/client::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/internal::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=37060 exit_code=1 stdout_bytes=854 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\flags
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/internal/flags cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\flags available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/internal/client/dev cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dev concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=5004 exit_code=1 stdout_bytes=1361 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/internal::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=17796 exit_code=1 stdout_bytes=17386 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\server
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/internal/server cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\server available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=playgrounds/sandbox cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\playgrounds\sandbox concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=38952 exit_code=1 stdout_bytes=20021 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=biome scope=packages/svelte/src/action cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\action concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=biome pid=20356 exit_code=1 stdout_bytes=11522 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dev
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/internal/client/dev cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dev available_permits=1
[OK] Sandbox: processo efemero concluido command=biome pid=21348 exit_code=1 stdout_bytes=1824 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\action
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=packages/svelte/src/action cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\action available_permits=2
[OK] Sandbox: processo efemero concluido command=biome pid=17344 exit_code=1 stdout_bytes=9173 stderr_bytes=449 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\playgrounds\sandbox
[PROC] SAST monorepo: sub-scan concluído blade=biome scope=playgrounds/sandbox cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\playgrounds\sandbox available_permits=3
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=270
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=.::files-01 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte concurrency_limit=3 in_flight=1
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/compiler cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\compiler concurrency_limit=3 in_flight=2
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\scripts concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=19864 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\compiler
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/compiler cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\compiler available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/animate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\animate concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=11876 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=.::files-01 cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler/migrate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\migrate concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=840 exit_code=0 stdout_bytes=5956 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\scripts
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\scripts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/attachments cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\attachments concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=12384 exit_code=0 stdout_bytes=191 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\animate
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/animate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\animate available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler/phases/1-parse cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\1-parse concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=43712 exit_code=0 stdout_bytes=588 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\migrate
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler/migrate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\migrate available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler/phases/2-analyze cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\2-analyze concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=12644 exit_code=0 stdout_bytes=202 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\attachments
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/attachments cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\attachments available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler/phases::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=14500 exit_code=0 stdout_bytes=4141 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\1-parse
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler/phases/1-parse cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\1-parse available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler/phases/3-transform cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\3-transform concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=12352 exit_code=0 stdout_bytes=2864 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\2-analyze
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler/phases/2-analyze cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\2-analyze available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler/preprocess cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\preprocess concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=15688 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler/phases::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler/print cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\print concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=41192 exit_code=0 stdout_bytes=4620 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\3-transform
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler/phases/3-transform cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\3-transform available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler/utils cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\utils concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=15088 exit_code=0 stdout_bytes=1149 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\preprocess
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler/preprocess cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\preprocess available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=8284 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\print
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler/print cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\print available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/easing cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\easing concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=21032 exit_code=0 stdout_bytes=1051 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\utils
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler/utils cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\utils available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/events cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\events concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=14860 exit_code=0 stdout_bytes=620 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/internal/client/dev cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dev concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=44024 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\easing
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/easing cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\easing available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/internal/client/dom cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=13048 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\events
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/events cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\events available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/internal/client/reactivity cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\reactivity concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=40908 exit_code=0 stdout_bytes=588 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dev
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/internal/client/dev cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dev available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/internal/client::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=3636 exit_code=0 stdout_bytes=3990 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/internal/client/dom cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/compiler/types cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\types concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=9108 exit_code=0 stdout_bytes=2239 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\reactivity
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/internal/client/reactivity cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\reactivity available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/internal/flags cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\flags concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=33304 exit_code=0 stdout_bytes=989 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/internal/client::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/internal/server cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\server concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=23884 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\types
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/compiler/types cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\types available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/internal::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=45484 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\flags
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/internal/flags cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\flags available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=playgrounds/sandbox cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\playgrounds\sandbox concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=37216 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal
[OK] Sandbox: processo efemero concluido command=oxlint pid=32744 exit_code=0 stdout_bytes=1530 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\server
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/internal::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal available_permits=0
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/internal/server cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\server available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src/action cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\action concurrency_limit=3 in_flight=3
[PROC] SAST monorepo: permissão adquirida blade=oxc scope=packages/svelte/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=43984 exit_code=0 stdout_bytes=598 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\playgrounds\sandbox
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=playgrounds/sandbox cwd=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\playgrounds\sandbox available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\scripts concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=36408 exit_code=0 stdout_bytes=193 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\action
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src/action cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\action available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/action cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\action concurrency_limit=3 in_flight=3
[OK] Sandbox: processo efemero concluido command=oxlint pid=14024 exit_code=0 stdout_bytes=3290 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src
[PROC] SAST monorepo: sub-scan concluído blade=oxc scope=packages/svelte/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/animate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\animate concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=300
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=795 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=330
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=360
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=390
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=420
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=450
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=480
[OK] Sandbox: processo efemero concluido command=opengrep pid=35444 exit_code=0 stdout_bytes=143 stderr_bytes=5998 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\animate
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/animate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\animate available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/attachments cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\attachments concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=21724 exit_code=0 stdout_bytes=1653 stderr_bytes=6124 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\action
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/action cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\action available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler/migrate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\migrate concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=510
[OK] Sandbox: processo efemero concluido command=opengrep pid=36096 exit_code=0 stdout_bytes=177347 stderr_bytes=6351 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\scripts
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/scripts cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\scripts available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler/phases/1-parse cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\1-parse concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=540
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=570
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=600
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=630
[OK] Sandbox: processo efemero concluido command=opengrep pid=25352 exit_code=0 stdout_bytes=143 stderr_bytes=2548 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\attachments
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/attachments cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\attachments available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler/phases/2-analyze cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\2-analyze concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=660
[OK] Sandbox: processo efemero concluido command=opengrep pid=39372 exit_code=0 stdout_bytes=201013 stderr_bytes=2225 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\migrate
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler/migrate cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\migrate available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler/phases/3-transform cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\3-transform concurrency_limit=3 in_flight=3
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=690
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=720
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=750
[OK] Sandbox: processo efemero concluido command=opengrep pid=8252 exit_code=0 stdout_bytes=300901 stderr_bytes=2080 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\1-parse
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler/phases/1-parse cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\1-parse available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler/phases::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=780
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=810
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=840
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=870
[OK] Sandbox: processo efemero concluido command=opengrep pid=43176 exit_code=0 stdout_bytes=646850 stderr_bytes=1602 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\2-analyze
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler/phases/2-analyze cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\2-analyze available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler/print cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\print concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=27384 exit_code=0 stdout_bytes=119404 stderr_bytes=1349 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler/phases::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler/types cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\types concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=16420 exit_code=0 stdout_bytes=714146 stderr_bytes=1504 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\3-transform
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler/phases/3-transform cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\phases\3-transform available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler/utils cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\utils concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=900
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=930
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=960
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=990
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1020
[OK] Sandbox: processo efemero concluido command=opengrep pid=16584 exit_code=0 stdout_bytes=4919 stderr_bytes=2276 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\types
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler/types cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\types available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=45656 exit_code=0 stdout_bytes=74617 stderr_bytes=2249 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\print
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler/print cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\print available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/easing cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\easing concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1050
[OK] Sandbox: processo efemero concluido command=opengrep pid=15804 exit_code=0 stdout_bytes=112310 stderr_bytes=2279 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\utils
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler/utils cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\utils available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/events cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\events concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1080
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1110
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1140
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1170
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1200
[OK] Sandbox: processo efemero concluido command=opengrep pid=34364 exit_code=0 stdout_bytes=129 stderr_bytes=2374 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\easing
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/easing cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\easing available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/compiler/preprocess cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\preprocess concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=25964 exit_code=0 stdout_bytes=143 stderr_bytes=2698 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\events
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/events cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\events available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/internal/client/dev cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dev concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1230
[OK] Sandbox: processo efemero concluido command=opengrep pid=9952 exit_code=0 stdout_bytes=85029 stderr_bytes=2499 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/internal/client/dom cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1260
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1290
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1320
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1350
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1380
[OK] Sandbox: processo efemero concluido command=opengrep pid=13104 exit_code=0 stdout_bytes=33971 stderr_bytes=1499 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\preprocess
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/compiler/preprocess cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\compiler\preprocess available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/internal/client/reactivity cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\reactivity concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1410
[OK] Sandbox: processo efemero concluido command=opengrep pid=33964 exit_code=0 stdout_bytes=107589 stderr_bytes=1379 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dev
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/internal/client/dev cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dev available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/internal/client::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[ERR] Sandbox: idle timeout atingido; aniquilando sidecar command=opengrep pid=15356 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom idle_timeout_secs=180 absolute_timeout_secs=600
[ERR] Sandbox: sidecar aniquilado apos timeout command=opengrep pid=15356 stdout_bytes=0 stderr_bytes=708 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom timeout_kind=idle
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/internal/client/dom cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\dom available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/internal/flags cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\flags concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1440
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1470
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1500
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1530
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1560
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1590
[OK] Sandbox: processo efemero concluido command=opengrep pid=40896 exit_code=0 stdout_bytes=165 stderr_bytes=2026 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\flags
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/internal/flags cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\flags available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/internal/server cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\server concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=33504 exit_code=0 stdout_bytes=243951 stderr_bytes=2552 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\reactivity
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/internal/client/reactivity cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client\reactivity available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src/internal::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[OK] Sandbox: processo efemero concluido command=opengrep pid=42348 exit_code=0 stdout_bytes=249630 stderr_bytes=1752 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/internal/client::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\client available_permits=0
[PROC] SAST monorepo: permissão adquirida blade=opengrep scope=packages/svelte/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src concurrency_limit=3 in_flight=3
[PROC] Semgrep: ruleset air-gapped materializado repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte rule_set=Health copied_rule_files=0 workspace_rules_dir=Z:\genesis_mc\src-tauri\semgrep\rules support_dir=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\.soda_semgrep\svelte\health
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1620
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1650
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1680
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1710
[OK] Sandbox: processo efemero concluido command=opengrep pid=19884 exit_code=0 stdout_bytes=1135 stderr_bytes=1547 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/internal::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal available_permits=1
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1740
[OK] Sandbox: processo efemero concluido command=opengrep pid=16648 exit_code=0 stdout_bytes=117369 stderr_bytes=1551 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\server
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src/internal/server cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src\internal\server available_permits=2
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1770
[PROC] F0: heartbeat repo_id=sveltejs/svelte elapsed_s=1800
[ERR] Sandbox: idle timeout atingido; aniquilando sidecar command=opengrep pid=30924 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src idle_timeout_secs=180 absolute_timeout_secs=600
[ERR] Sandbox: sidecar aniquilado apos timeout command=opengrep pid=30924 stdout_bytes=0 stderr_bytes=0 repo_path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src timeout_kind=idle
[PROC] SAST monorepo: sub-scan concluído blade=opengrep scope=packages/svelte/src::files-01 cwd=\\?\C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte\packages\svelte\src available_permits=3
[FINOPS] N11: roteador poliglota de SAST concluido repo_id=sveltejs/svelte elapsed_ms=1565325 unsafe_hotspots_bytes=5318 health_report_bytes=172283
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_06_unsafe_hotspots payload_bytes=5318
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_08_health_report payload_bytes=172283
[PROC] N10: Finalizando coleta de metadados comunitarios repo_id=sveltejs/svelte
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_09_community_meta payload_bytes=2167
[PROC] N11: Extraindo blob_10_soda_canon_context repo_id=sveltejs/svelte
[PROC] blob_10_soda_canon_context servido do cache SQLite repo_id=sveltejs/svelte
[PROC] Blob gerado repo_id=sveltejs/svelte artifact_type=blob_10_soda_canon_context payload_bytes=4648
[PROC] N12: Persistindo pacote RAW no SQLite repo_id=sveltejs/svelte blobs_count=11 total_payload_bytes=886136
[OK] N12: Persistencia do pacote RAW concluida repo_id=sveltejs/svelte
[OK] N13: pipeline_core retornou; iniciando teardown repo_id=sveltejs/svelte is_ok=true
[PROC] N13: PurgeGuard iniciando limpeza atomica (Sandbox + TempWorkspace) repo_id=sveltejs/svelte
[PROC] PurgeGuard: Iniciando limpeza atômica de recursos
[PROC] PurgeGuard: SandboxHandle descartado
[PROC] RamdiskHandle: iniciando teardown ProjFS path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500 projected_roots=1
[FINOPS] RamdiskHandle: virtualization root delegada para delecao externa path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500\repos\sveltejs\svelte elapsed_ms=76
[FINOPS] RamdiskHandle: cleanup explicito concluido com delecao externa não-bloqueante path=C:\Users\rosas\AppData\Local\Temp\.souls_workspaces\souls_mc_workspace_6604_1782284101719926500 elapsed_ms=112
[PROC] RamdiskGuard: cleanup ja delegado externamente; Drop nao repetira a remocao
[PROC] PurgeGuard: RamdiskHandle descartado
[PROC] N13: Teardown finalizado; retornando ao CLI repo_id=sveltejs/svelte
[FINOPS] F0: concluído repo_id=sveltejs/svelte row_number=768 report=Z:\genesis_mc\.soda_scratchpad\reports\_ETL_REPORT_sveltejs_svelte.txt elapsed_ms=1815210
[ERR] F0(batch): resumo final total_candidates=5 ok=4 error_count=1 skipped=0 total_elapsed_ms=5690449 avg_ms=1132734
[FINOPS] F0(batch): OK repo_id=mendableai/firecrawl row_number=306 elapsed_ms=1025192 blobs=11 missing=0 missing_list=[] report=Some("Z:\\genesis_mc\\.soda_scratchpad\\reports\\_ETL_REPORT_mendableai_firecrawl.txt")
[FINOPS] F0(batch): OK repo_id=huggingface/candle row_number=365 elapsed_ms=2319631 blobs=11 missing=0 missing_list=[] report=Some("Z:\\genesis_mc\\.soda_scratchpad\\reports\\_ETL_REPORT_huggingface_candle.txt")
[ERR] F0(batch): ERRO repo_id=agentjido/jido row_number=394 elapsed_ms=15222 blobs=10 missing=1 missing_list=["blob_10_soda_canon_context"] error=Some("Extraction failed: Execution failed: Nenhum símbolo estrutural foi extraído do repositório '\\\\?\\C:\\Users\\rosas\\AppData\\Local\\Temp\\.souls_workspaces\\souls_mc_workspace_6604_1782283587387901200\\repos\\agentjido\\jido'")
[FINOPS] F0(batch): OK repo_id=multigres/multigres-operator row_number=562 elapsed_ms=488418 blobs=11 missing=0 missing_list=[] report=Some("Z:\\genesis_mc\\.soda_scratchpad\\reports\\_ETL_REPORT_multigres_multigres-operator.txt")
[FINOPS] F0(batch): OK repo_id=sveltejs/svelte row_number=768 elapsed_ms=1815210 blobs=11 missing=0 missing_list=[] report=Some("Z:\\genesis_mc\\.soda_scratchpad\\reports\\_ETL_REPORT_sveltejs_svelte.txt")
[PROC] F0(batch): concluído
PS Z:\genesis_mc\src-tauri>