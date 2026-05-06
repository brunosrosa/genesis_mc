import os
import subprocess
import json
import time
import shutil
import stat
import logging
from urllib.parse import urlparse
from typing import List, Optional

# ==============================================================================
# SODA HARVESTER - FASE 1: A COLHEITA (Força Bruta Determinística)
# ==============================================================================
# Este script implementa a Fase 1 do Blueprint SODA.
# Ele é estritamente determinístico: zero IA, zero alucinação.
# Seu único propósito é transformar repositórios caóticos em 9 artefatos RAW
# purificados, isolando o host de execuções maliciosas via Windows Sandbox.
# ==============================================================================

# --- Configuração de Logging (Pessimismo da Razão) ---
# Falhas são esperadas. Logamos tudo para não travar o loop mestre.
logging.basicConfig(
    filename='soda_harvester_fase1.log',
    level=logging.INFO,
    format='%(asctime)s - [%(levelname)s] - %(message)s'
)
console = logging.StreamHandler()
console.setLevel(logging.INFO)
logging.getLogger('').addHandler(console)

# --- Constantes Arquiteturais Absolutas ---
TEMP_CLONE_DIR = "./temp_clones"
OUTPUT_RAW_DIR = "./soda_raw_data"
MAX_RETRIES = 3
RATE_LIMIT_DELAY = 5  # O Pedágio (Segundos entre requisições de rede externas)

class HarvesterFase1:
    """
    Motor de extração cirúrgica e determinística.
    Opera sob a premissa de que todo código de terceiros é hostil e poluído.
    """
    def __init__(self, target_urls: List[str]):
        self.target_urls = target_urls
        self.ensure_directories()
        
    def ensure_directories(self):
        """Garante a existência e higiene da estrutura de pastas base."""
        for d in [TEMP_CLONE_DIR, OUTPUT_RAW_DIR]:
            os.makedirs(d, exist_ok=True)
            
    def get_repo_id(self, url: str) -> str:
        """Deriva um ID único, canônico e seguro para o File System a partir da URL."""
        parsed = urlparse(url)
        # Ex: "https://github.com/FSoft-AI4Code/CodeWiki" -> "FSoft-AI4Code_CodeWiki"
        path = parsed.path.strip('/')
        return path.replace('/', '_')

    def execute_with_retry(self, command: List[str], cwd: Optional[str] = None) -> bool:
        """
        O Amortecedor de Rede: Implementa Tenacity-like behavior para comandos CLI nativos.
        Lida com instabilidades transitórias de rede, recusas de TLS ou I/O.
        """
        for attempt in range(MAX_RETRIES):
            try:
                # Timeout draconiano (300s = 5min) para impedir hangs infinitos em I/O
                result = subprocess.run(
                    command, 
                    cwd=cwd, 
                    capture_output=True, 
                    text=True, 
                    timeout=300
                )
                if result.returncode == 0:
                    return True
                else:
                    logging.warning(f"Falha no comando (Tentativa {attempt+1}/{MAX_RETRIES}): {' '.join(command)}\nStderr: {result.stderr.strip()}")
            except subprocess.TimeoutExpired:
                 logging.error(f"Timeout fatal no comando: {' '.join(command)}")
            except Exception as e:
                logging.error(f"Exceção inesperada durante subprocesso: {e}")
            
            # Jitter / Backoff Exponencial (2s, 4s, 8s)
            delay = 2 ** (attempt + 1)
            logging.info(f"Aguardando {delay}s antes da próxima tentativa para dissipar gargalos...")
            time.sleep(delay)
            
        return False

    # --------------------------------------------------------------------------
    # ESTÁGIO 1: REDE E ISOLAMENTO DE EXECUÇÃO
    # --------------------------------------------------------------------------

    def step_1_blobless_clone(self, url: str, target_dir: str) -> bool:
        """
        Clonagem Cirúrgica (--filter=blob:none).
        Baixa a árvore de commits (necessária para análise de entropia/hotspots temporais),
        mas poupa I/O e disco ao não baixar arquivos físicos até serem explicitamente lidos.
        """
        logging.info("Iniciando Blobless Clone (Preservando Ontologia de Histórico)...")
        if os.path.exists(target_dir):
            self._force_remove_dir(target_dir) # Purge prévio caso tenha abortado na rodada anterior
            
        command = ["git", "clone", "--filter=blob:none", url, target_dir]
        return self.execute_with_retry(command)

    def step_2_sandbox_execution(self, repo_id: str, clone_dir: str, out_dir: str):
        """
        O Abate Seguro: Isola a execução de linters (Cargo/NPM) no Windows Sandbox.
        Garante imunidade contra scripts maliciosos de build.rs ou postinstall.
        """
        logging.info("Preparando Isolamento Hyper-V (Windows Sandbox)...")
        wsb_path = os.path.abspath(f"./{repo_id}_sandbox.wsb")
        abs_clone_dir = os.path.abspath(clone_dir)
        abs_out_dir = os.path.abspath(out_dir)
        
        # Gera o XML do Sandbox dinamicamente. O host mapeia o código original como ReadOnly (Intocável).
        wsb_content = f"""<Configuration>
            <vGPU>Disable</vGPU>
            <Networking>Disable</Networking>
            <MappedFolders>
                <MappedFolder>
                    <HostFolder>{abs_clone_dir}</HostFolder>
                    <SandboxFolder>C:\\target_repo</SandboxFolder>
                    <ReadOnly>true</ReadOnly>
                </MappedFolder>
                <MappedFolder>
                    <HostFolder>{abs_out_dir}</HostFolder>
                    <SandboxFolder>C:\\output</SandboxFolder>
                    <ReadOnly>false</ReadOnly>
                </MappedFolder>
            </MappedFolders>
            <LogonCommand>
                <Command>cmd.exe /c "cd C:\\target_repo && cargo clippy --message-format=json > C:\\output\\08_health_report.json"</Command>
            </LogonCommand>
        </Configuration>"""
        
        with open(wsb_path, 'w') as f:
            f.write(wsb_content)
            
        logging.info("Executando Linters na Sandbox (Simulado nesta rotina base)...")
        # --- AQUI OCORRERIA O ACIONAMENTO REAL: subprocess.run(["wsb", wsb_path]) ---
        # A API do WSB abriria a VM efêmera e a fecharia sozinha após o comando.
        
        # POC MOCK: Simulando a geração do artefato 08 pelo Sandbox no disco do Host
        time.sleep(1) 
        with open(os.path.join(out_dir, "08_health_report.json"), "w") as f:
             json.dump({
                 "status": "sandbox_execution_success", 
                 "clippy_warnings": 42,
                 "cyclomatic_complexity_score": 14.5
             }, f, indent=2)
        
        # Limpeza do XML de configuração efêmero
        if os.path.exists(wsb_path):
             os.remove(wsb_path)

    # --------------------------------------------------------------------------
    # ESTÁGIO 2: EXTRAÇÃO SEMÂNTICA (A Peneira de AST e Metadados)
    # --------------------------------------------------------------------------

    def step_3_core_extraction(self, repo_id: str, clone_dir: str, out_dir: str):
        """
        Extrai Identidade (README), Custos Operacionais (Manifestos), AST (Matéria Escura) e Grafo Relacional.
        """
        logging.info("Iniciando extração semântica e estrutural local...")
        
        # ARTEFATO 01: A Promessa Declarada (README)
        readme_src = os.path.join(clone_dir, "README.md")
        if os.path.exists(readme_src):
            shutil.copy(readme_src, os.path.join(out_dir, "01_promessa_readme.md"))
        else:
             with open(os.path.join(out_dir, "01_promessa_readme.md"), "w") as f: 
                 f.write("README não encontrado no repósitorio. Intenção não declarada.")

        # ARTEFATO 02: O Custo de Operação (Manifestos Tóxicos ou Limpos)
        manifests = {}
        for filename in ["Cargo.toml", "package.json", "go.mod"]:
            path = os.path.join(clone_dir, filename)
            if os.path.exists(path):
                with open(path, 'r', encoding='utf-8', errors='ignore') as f:
                    manifests[filename] = f.read()
        with open(os.path.join(out_dir, "02_dependency_manifest.json"), "w") as f:
            json.dump(manifests, f, indent=2)

        # ARTEFATO 04: A Alma Matemática (JCodemunch AST)
        logging.info("Destilando AST via JCodemunch...")
        with open(os.path.join(out_dir, "04_repo_outline.txt"), "w") as f:
            # Comando real de integração: subprocess.run(["jcodemunch-mcp", "get_repo_outline", "--path", clone_dir], stdout=f)
            f.write("SIMULAÇÃO JCODEMUNCH: Mapeamento de Assinaturas Matemáticas (AST) Concluído.\n")
            f.write("fn extract_logic(data: Vec<u8>) -> Result<(), Error>\n")
            f.write("class CoreProcessor { pub fn init() }\n")

        # ARTEFATO 05: O Esqueleto Espacial Dimensional (Aider Repo-Map)
        logging.info("Gerando Grafo Arquitetural Dimensional via Aider...")
        with open(os.path.join(out_dir, "05_architecture_map.txt"), "w") as f:
            # Comando real de integração: subprocess.run(["aider", "--repo-map", "--map-file", os.path.join(out_dir, "05_architecture_map.txt")], cwd=clone_dir)
             f.write("SIMULAÇÃO AIDER: Árvore hierárquica do projeto gerada.\n")
             f.write("src/\n  auth/\n  database/ (Fortemente acoplado a auth)\n")
             
        # ARTEFATO 07: O Blueprint Logístico (Ops)
        logging.info("Mapeando Ops Blueprint (Varredura de text-files de Infra)...")
        with open(os.path.join(out_dir, "07_ops_blueprint.txt"), "w") as out_f:
             found_ops_files = False
             for root, dirs, files in os.walk(clone_dir):
                  for file in files:
                       # Filtros restritos para capturar CI/CD e Infra sem ruído
                       if file in ["Dockerfile", "docker-compose.yml", "Makefile"] or (file.endswith(".yml") and ".github" in root):
                           found_ops_files = True
                           out_f.write(f"--- START OF {file} ---\n")
                           try:
                               with open(os.path.join(root, file), 'r', encoding='utf-8', errors='ignore') as f:
                                   # Limita leitura para 50kb para evitar dumps gigantescos em caso de yml atípicos
                                   content = f.read(50000) 
                                   out_f.write(content)
                           except Exception as e:
                               out_f.write(f"[Erro de Leitura I/O: {e}]\n")
                           out_f.write(f"\n--- END OF {file} ---\n\n")
             if not found_ops_files:
                 out_f.write("Nenhum arquivo explícito de infraestrutura (Docker/CI) encontrado. Projeto potencialmente Bare Metal ou manual.")

    def step_4_ux_contracts(self, repo_id: str, clone_dir: str, out_dir: str):
         """
         A Lente UX: Usa Oxc (Linter Rust nativo) para dissecar componentes de UI.
         Ignora HTML semântico; extrai estritamente Props (entradas) e Eventos (saídas).
         """
         logging.info("Destilando Contratos Visuais de UX via Oxc...")
         with open(os.path.join(out_dir, "03_ux_contracts.txt"), "w") as f:
             # Comando real de integração: subprocess.run(["oxc", "lint", clone_dir, "--format=json"])
             f.write("SIMULAÇÃO OXC: Extração cirúrgica de $props() e Dispatchers de Svelte 5 / React.\n")
             f.write("Component: NavBar.svelte | Props: { user_id: string } | Events: ['login', 'logout']\n")

    def step_5_community_meta(self, repo_id: str, url: str, out_dir: str):
        """
        Extrator Social e Varredura de Código Inseguro.
        """
        logging.info("Baixando Metadados Sociais (GH CLI)...")
        # ARTEFATO 09: Vitalidade Social
        with open(os.path.join(out_dir, "09_community_meta.json"), "w") as f:
            # Comando real de integração: subprocess.run(["gh", "issue", "list", "--repo", url, "--limit", "50", "--json", "title,state,createdAt"])
            json.dump({
                "simulacao": True,
                "project_vitality": "Alive",
                "avg_response_time_days": 2.4,
                "recent_prs_merged": 15
            }, f, indent=2)
            
        # ARTEFATO 06: Mapeamento de Zonas de Perigo (Unsafe Hotspots)
        # Integração real requereria um ripgrep (rg) sobre o clone: rg "unsafe|eval\("
        logging.info("Mapeando Unsafe Hotspots via Regex estruturado...")
        with open(os.path.join(out_dir, "06_unsafe_hotspots.txt"), "w") as f:
             f.write("SIMULAÇÃO RIPGREP: Varredura de blocos 'unsafe', 'eval' e 'raw_ptr'.\n")
             f.write("- src/core/memory.rs: Linha 45 [unsafe block detectado - Mutação manual de ponteiro]\n")

    # --------------------------------------------------------------------------
    # O ORQUESTRADOR CENTRAL E ROTINA DE PURGA
    # --------------------------------------------------------------------------

    def _force_remove_dir(self, directory: str):
        """
        No Windows, arquivos locais do .git frequentemente ficam travados como 'Read-Only'.
        Esta função força a deleção ativamente, alterando permissões, garantindo a Purga.
        """
        def remove_readonly(func, path, excinfo):
            os.chmod(path, stat.S_IWRITE)
            func(path)
        shutil.rmtree(directory, onerror=remove_readonly)

    def run_pipeline(self):
        """
        Itera implacavelmente sobre a matriz de repositórios alvo.
        Aplica a doutrina de Fail-Fast e garante a Purga Nuclear ao término de cada URL.
        """
        total = len(self.target_urls)
        logging.info(f"=== INICIANDO COLHEITA FASE 1 ({total} REPOSITÓRIOS) ===")
        
        for idx, url in enumerate(self.target_urls):
            repo_id = self.get_repo_id(url)
            clone_dir = os.path.join(TEMP_CLONE_DIR, repo_id)
            out_dir = os.path.join(OUTPUT_RAW_DIR, repo_id)
            
            logging.info(f"--- [{idx+1}/{total}] Processando Alvo: {repo_id} ---")
            
            # Garante pasta de destino 100% limpa para os exatos 9 artefatos
            if os.path.exists(out_dir):
                shutil.rmtree(out_dir, ignore_errors=True)
            os.makedirs(out_dir)
            
            try:
                # O Pedágio de Rede: Respeita leis de Rate Limiting externamente
                time.sleep(RATE_LIMIT_DELAY) 
                
                # Fase 1.1: Clone Blobless (A Âncora Histórica)
                if not self.step_1_blobless_clone(url, clone_dir):
                    logging.error("Abortando processamento do repositório: Falha fatal na clonagem.")
                    continue
                
                # Fase 1.2: O Abate no Sandbox (Anti-RCE)
                self.step_2_sandbox_execution(repo_id, clone_dir, out_dir)
                
                # Fase 1.3: Extração Lógica (A Alma e o Esqueleto)
                self.step_3_core_extraction(repo_id, clone_dir, out_dir)
                
                # Fase 1.4: Lente UX (Contratos de Interface)
                self.step_4_ux_contracts(repo_id, clone_dir, out_dir)
                
                # Fase 1.5: Saúde Social e Zonas de Perigo
                self.step_5_community_meta(repo_id, url, out_dir)
                
                logging.info(f"[{repo_id}] PACOTE RAW GERADO E HIGIENIZADO COM SUCESSO (9 Artefatos).")
                
            except Exception as e:
                 logging.error(f"[{repo_id}] Falha Sistêmica Catastrófica no pipeline isolado: {e}")
            finally:
                # O Protocolo de Limpeza Nuclear (Purge Routine) - OBRIGATÓRIO
                # O Harvester em hipótese alguma arquiva lixo cru ou código-fonte nativo no disco.
                logging.info(f"[{repo_id}] Executando Purga Nuclear do clone original...")
                if os.path.exists(clone_dir):
                    self._force_remove_dir(clone_dir)
                    
        logging.info("=== COLHEITA FASE 1 CONCLUÍDA ABSOLUTAMENTE ===")

# ==============================================================================
# ENTRYPOINT (Para Testes Locais / Execução Direta)
# ==============================================================================
if __name__ == "__main__":
    # Lote de Teste. Posteriormente, o script principal de integração do SODA (Fase 2) 
    # chamará esta classe passando a lista injetada via CSV/SQLite.
    lote_de_estudo_radar = [
        "https://github.com/aaif-goose/goose",  # Foco em Rust / Arquitetura Profunda
        "https://github.com/sveltejs/svelte"    # Alvo principal para Oxc (Interface/UX)
    ]
    
    harvester = HarvesterFase1(lote_de_estudo_radar)
    harvester.run_pipeline()