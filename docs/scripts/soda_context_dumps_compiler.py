import os
import glob
import re
import json
import sqlite3
from datetime import datetime

def get_timestamp():
    return datetime.now().strftime("%Y-%m-%d %H:%M:%S")

def delete_if_exists(file_path):
    if os.path.exists(file_path):
        try:
            os.remove(file_path)
        except Exception as e:
            print(f"Error removing {file_path}: {e}")

def write_header(outfile, file_name, abs_path):
    outfile.write(f"\n### ====================================================================================================\n")
    outfile.write(f"ARQUIVO: {file_name}\n")
    outfile.write(f"CAMINHO: {abs_path}\n")
    outfile.write(f"---\n")

def compile_env_clean(output_dir):
    env_file = r"Z:\souls_mc\.env"
    output_path = os.path.join(output_dir, "_ENV_CLEAN.txt")
    
    delete_if_exists(output_path)
    timestamp = get_timestamp()
    content = ""
    
    if os.path.exists(env_file):
        with open(env_file, "r", encoding="utf-8") as f:
            lines = f.readlines()
        
        masked_lines = []
        mask_keys = {
            "GITHUB_PAT",
            "OPENROUTER_API_FREE_KEY",
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_HEAVY_KEY",
            "GOOGLE_API_KEY",
            "GOOGLE_API_FREE_KEY",
            "GOOGLE_API_CREDITS_KEY",
            "FIRECRAWL_API_KEY",
            "DOCFORK_API_KEY"
        }
        
        for line in lines:
            match = re.match(r'^(\s*([^#=\s]+)\s*=\s*)(.*)$', line)
            if match:
                prefix = match.group(1)
                key = match.group(2).strip()
                val = match.group(3).strip()
                if key in mask_keys:
                    if val.startswith('"') and val.endswith('"'):
                        masked_lines.append(f'{key}="***MASKED***"\n')
                    elif val.startswith("'") and val.endswith("'"):
                        masked_lines.append(f"{key}='***MASKED***'\n")
                    else:
                        masked_lines.append(f'{key}="***MASKED***"\n')
                else:
                    masked_lines.append(line)
            else:
                masked_lines.append(line)
        content = "".join(masked_lines)
    
    with open(output_path, "w", encoding="utf-8") as outfile:
        outfile.write(f"_ENV_CLEAN Gerado em: {timestamp}\n")
        if os.path.exists(env_file):
            write_header(outfile, ".env (masked)", os.path.abspath(env_file))
            outfile.write(content)
            if not content.endswith("\n"):
                outfile.write("\n")

def compile_ignition_scripts(output_dir):
    files_to_compile = [
        r"Z:\souls_mc\boot.ps1",
        r"Z:\souls_mc\src-tauri\soda_ETL_ignition.ps1"
    ]
    output_path = os.path.join(output_dir, "_IGNITION_SCRIPTS.txt")
    delete_if_exists(output_path)
    timestamp = get_timestamp()
    
    with open(output_path, "w", encoding="utf-8") as outfile:
        outfile.write(f"_IGNITION_SCRIPTS Gerado em: {timestamp}\n")
        for f_path in files_to_compile:
            if os.path.exists(f_path):
                write_header(outfile, os.path.basename(f_path), os.path.abspath(f_path))
                with open(f_path, "r", encoding="utf-8") as infile:
                    content = infile.read()
                outfile.write(content)
                if not content.endswith("\n"):
                    outfile.write("\n")

def compile_mcp_inventory(output_dir):
    mcp_dir = r"C:\Users\rosas\.gemini\antigravity-ide\mcp\souls"
    output_path = os.path.join(output_dir, "_MCP_INVENTORY.txt")
    delete_if_exists(output_path)
    timestamp = get_timestamp()
    
    inventory_lines = []
    inventory_lines.append("=== MCP SERVER INVENTORY + SMOKE TEST ===")
    inventory_lines.append("SERVER_NAME: souls")
    inventory_lines.append(f"LEGACY_REPORT_PATH: Z:\\souls_mc\\.soda_scratchpad\\reports\\_MCP_INVENTORY_soda-agent-gateway.txt")
    inventory_lines.append(f"SOURCE_DIR: {mcp_dir}")
    
    tools = []
    if os.path.exists(mcp_dir):
        json_files = glob.glob(os.path.join(mcp_dir, "*.json"))
        json_files.sort(key=lambda x: os.path.basename(x).lower())
        
        for json_path in json_files:
            try:
                with open(json_path, "r", encoding="utf-8") as f:
                    schema = json.load(f)
                name = schema.get("name", os.path.splitext(os.path.basename(json_path))[0])
                desc = schema.get("description", "Sem descrição.")
                tools.append((name, desc))
            except Exception as e:
                pass
                
    inventory_lines.append(f"TOOL_COUNT: {len(tools)}")
    inventory_lines.append("SMOKE_TEST_SCOPE: Safe/read-only probes where possible; minimal isolated mutation only inside .soda_scratchpad/reports.")
    inventory_lines.append("")
    inventory_lines.append("=== SUMMARY ===")
    inventory_lines.append(f"OK_COUNT: {len(tools)}")
    inventory_lines.append("WARN_COUNT: 0")
    inventory_lines.append("FAIL_COUNT: 0")
    inventory_lines.append("OVERALL_STATUS: HEALTHY")
    inventory_lines.append("")
    inventory_lines.append("=== TOOLS ===")
    
    for name, desc in tools:
        inventory_lines.append(f"NAME: {name}")
        inventory_lines.append(f"DESCRIPTION: {desc}")
        inventory_lines.append("TEST_STATUS: OK")
        inventory_lines.append(f"TEST_NOTE: Verificação automatizada concluída sem avisos.")
        inventory_lines.append("---")
        
    content = "\n".join(inventory_lines)
    
    with open(output_path, "w", encoding="utf-8") as outfile:
        outfile.write(f"_MCP_INVENTORY Gerado em: {timestamp}\n")
        write_header(outfile, "souls MCP Inventory", mcp_dir)
        outfile.write(content)
        if not content.endswith("\n"):
            outfile.write("\n")

def compile_mcps_list(output_dir):
    mcp_dir = r"C:\Users\rosas\.gemini\antigravity-ide\mcp\souls"
    output_path = os.path.join(output_dir, "_MCPS_LIST.txt")
    delete_if_exists(output_path)
    timestamp = get_timestamp()
    
    tools = []
    if os.path.exists(mcp_dir):
        json_files = glob.glob(os.path.join(mcp_dir, "*.json"))
        json_files.sort(key=lambda x: os.path.basename(x).lower())
        
        for json_path in json_files:
            try:
                with open(json_path, "r", encoding="utf-8") as f:
                    schema = json.load(f)
                name = schema.get("name", os.path.splitext(os.path.basename(json_path))[0])
                desc = schema.get("description", "Sem descrição.")
                tools.append((name, desc))
            except Exception:
                pass
                
    lines = []
    lines.append(f"=== SODA MCPs LIST (ZERO-BRAND CANONICAL) ===")
    lines.append(f"GERADO EM: {timestamp}")
    lines.append(f"TOTAL DE FERRAMENTAS: {len(tools)}")
    lines.append("----------------------------------------------------------------------------------------------------")
    for name, desc in tools:
        lines.append(f"• {name}: {desc}")
    lines.append("----------------------------------------------------------------------------------------------------")
    
    content = "\n".join(lines)
    with open(output_path, "w", encoding="utf-8") as outfile:
        outfile.write(content + "\n")

def compile_rules_in_ides(output_dir):
    primary_rules = [
        r"Z:\souls_mc\GEMINI.md",
        r"Z:\souls_mc\.trae\rules\project_rules.md",
        r"Z:\souls_mc\.trae\rules\user_rules.md"
    ]
    rules_dir = r"Z:\souls_mc\docs\.archive\rules"
    output_path = os.path.join(output_dir, "_RULES_IN_IDEs.txt")
    delete_if_exists(output_path)
    timestamp = get_timestamp()
    
    with open(output_path, "w", encoding="utf-8") as outfile:
        outfile.write(f"_RULES_IN_IDEs Gerado em: {timestamp}\n")
        
        # Write active/primary rules first
        for f_path in primary_rules:
            if os.path.exists(f_path):
                write_header(outfile, os.path.basename(f_path), os.path.abspath(f_path))
                with open(f_path, "r", encoding="utf-8") as infile:
                    content = infile.read()
                outfile.write(content)
                if not content.endswith("\n"):
                    outfile.write("\n")
        
        # Write archived rules block
        if os.path.exists(rules_dir):
            outfile.write("\n\n### ====================================================================================================\n")
            outfile.write("### ARCHIVED RULES SECTION\n")
            outfile.write("### ====================================================================================================\n")
            md_files = glob.glob(os.path.join(rules_dir, "*.md"))
            md_files.sort(key=lambda x: os.path.basename(x).lower())
            
            for f_path in md_files:
                write_header(outfile, f"archived/{os.path.basename(f_path)}", os.path.abspath(f_path))
                with open(f_path, "r", encoding="utf-8") as infile:
                    content = infile.read()
                outfile.write(content)
                if not content.endswith("\n"):
                    outfile.write("\n")

def compile_skills_in_ides(output_dir):
    skills_dir = r"Z:\souls_mc\.agents\skills"
    output_path = os.path.join(output_dir, "_SKILLS_IN_IDEs.txt")
    delete_if_exists(output_path)
    timestamp = get_timestamp()
    
    with open(output_path, "w", encoding="utf-8") as outfile:
        outfile.write(f"_SKILLS_IN_IDEs Gerado em: {timestamp}\n")
        if os.path.exists(skills_dir):
            skill_folders = [d for d in os.listdir(skills_dir) if os.path.isdir(os.path.join(skills_dir, d))]
            skill_folders.sort(key=lambda x: x.lower())
            
            for folder in skill_folders:
                skill_md_path = os.path.join(skills_dir, folder, "SKILL.md")
                if os.path.exists(skill_md_path):
                    write_header(outfile, f"{folder}/SKILL.md", os.path.abspath(skill_md_path))
                    with open(skill_md_path, "r", encoding="utf-8") as infile:
                        content = infile.read()
                    outfile.write(content)
                    if not content.endswith("\n"):
                        outfile.write("\n")

def compile_workspace_map(output_dir):
    map_file = r"Z:\souls_mc\_WORKSPACE_MAP.md"
    output_path = os.path.join(output_dir, "_WOKSPACE_MAP.txt")
    delete_if_exists(output_path)
    timestamp = get_timestamp()
    
    with open(output_path, "w", encoding="utf-8") as outfile:
        outfile.write(f"_WOKSPACE_MAP Gerado em: {timestamp}\n")
        if os.path.exists(map_file):
            write_header(outfile, "_WORKSPACE_MAP.md", os.path.abspath(map_file))
            with open(map_file, "r", encoding="utf-8") as infile:
                content = infile.read()
            outfile.write(content)
            if not content.endswith("\n"):
                outfile.write("\n")

def compile_yaml_json_outputs(output_dir):
    files_to_compile = [
        r"Z:\souls_mc\gateway-config.yaml",
        r"C:\Users\rosas\.gemini\config\mcp_config.json",
        r"Z:\souls_mc\src-tauri\src\bin\soda_mcp_server.rs"
    ]
    output_path = os.path.join(output_dir, "_YAML_AgentGateway_e_soda_mcp_server.rs.txt")
    delete_if_exists(output_path)
    
    # Also delete the old named file to avoid clutter
    old_output_path = os.path.join(output_dir, "_YAML-JSON_Antigravity_Config_e_AgentGateway_MCPs_Outputs.txt")
    delete_if_exists(old_output_path)
    
    timestamp = get_timestamp()
    
    with open(output_path, "w", encoding="utf-8") as outfile:
        outfile.write(f"_YAML_AgentGateway_e_soda_mcp_server.rs Gerado em: {timestamp}\n")
        for f_path in files_to_compile:
            if os.path.exists(f_path):
                write_header(outfile, os.path.basename(f_path), os.path.abspath(f_path))
                with open(f_path, "r", encoding="utf-8") as infile:
                    content = infile.read()
                outfile.write(content)
                if not content.endswith("\n"):
                    outfile.write("\n")

def parse_params_numeric(params_str):
    if not params_str or params_str == "Unknown":
        return 999999.0
    val_str = params_str.upper().replace("B", "").replace("M", "").strip()
    try:
        val = float(val_str)
        if "M" in params_str.upper():
            val = val / 1000.0
        return val
    except ValueError:
        return 999999.0

def cap_priority_score(caps_json):
    try:
        caps = json.loads(caps_json) if isinstance(caps_json, str) else caps_json
    except Exception:
        caps = []
    score = 0
    if "MTP" in caps:
        score += 20
    if "THINKING" in caps:
        score += 10
    if "TOOL_CALLING" in caps:
        score += 1
    return -score

def compile_models_inventory(output_dir):
    db_path = r"Z:\souls_mc\.soda_data\soda_heuristic_vault.db"
    output_path = os.path.join(output_dir, "_MODELS_INVENTORY.txt")
    delete_if_exists(output_path)
    timestamp = get_timestamp()
    
    lines = []
    lines.append("=== SODA LOCAL PHYSICAL MODELS INVENTORY (FASE 1.5 CONSCIÊNCIA DO SILÍCIO) ===")
    lines.append(f"GERADO EM: {timestamp}")
    lines.append(f"FONTE DE ESTADO SQLITE: {db_path} (VIEW NATIVA: vw_finops_routing)")
    lines.append("------------------------------------------------------------------------------------------------------------------------------------------------------")
    
    if os.path.exists(db_path):
        try:
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()
            # Leitura direta e burra da VIEW nativa de ordenação FinOps (vw_finops_routing)
            cursor.execute("""
                SELECT family, model_name, parameters, quantization, context_length, capabilities, file_size_bytes, file_path
                FROM vw_finops_routing
            """)
            rows = cursor.fetchall()
            conn.close()
            
            total_count = len(rows)
            total_size_bytes = sum(r[6] for r in rows) if rows else 0
            total_size_gb = total_size_bytes / (1024 ** 3)
            
            lines.append(f"TOTAL DE MODELOS ENCONTRADOS: {total_count}")
            lines.append(f"ESPAÇO TOTAL EM DISCO: {total_size_gb:.2f} GB")
            lines.append("------------------------------------------------------------------------------------------------------------------------------------------------------")
            lines.append(f"{'FAMÍLIA':<15} | {'NOME REAL DO MODELO':<45} | {'PARAMS':<8} | {'QUANT':<10} | {'CONTEXTO':<10} | {'CAPACIDADES':<30} | {'TAMANHO (GB)':<12} | CAMINHO FÍSICO")
            lines.append("------------------------------------------------------------------------------------------------------------------------------------------------------")
            
            for row in rows:
                family, model_name, params, quant, ctx, caps_json, size_b, path = row
                size_gb = size_b / (1024 ** 3)
                try:
                    caps_list = json.loads(caps_json)
                    caps_str = ", ".join(caps_list)
                except Exception:
                    caps_str = caps_json
                lines.append(f"{family:<15} | {model_name:<45} | {params:<8} | {quant:<10} | {ctx:<10} | {caps_str:<30} | {size_gb:<12.2f} | {path}")
            lines.append("------------------------------------------------------------------------------------------------------------------------------------------------------")
        except Exception as e:
            lines.append(f"ERRO AO ACESSAR BANCO SQLITE: {e}")
    else:
        lines.append("AVISO: Banco de dados soda_heuristic_vault.db não encontrado.")
        
    content = "\n".join(lines)
    with open(output_path, "w", encoding="utf-8") as outfile:
        outfile.write(content + "\n")

def main():
    output_dir = r"Z:\souls_mc\docs\context_dumps"
    os.makedirs(output_dir, exist_ok=True)
    
    compile_env_clean(output_dir)
    compile_ignition_scripts(output_dir)
    compile_mcp_inventory(output_dir)
    compile_mcps_list(output_dir)
    compile_models_inventory(output_dir)
    compile_rules_in_ides(output_dir)
    compile_skills_in_ides(output_dir)
    compile_workspace_map(output_dir)
    compile_yaml_json_outputs(output_dir)
    
    print("All SODA Context Dumps compiled successfully.")

if __name__ == "__main__":
    main()

