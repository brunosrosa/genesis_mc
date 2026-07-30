#!/usr/bin/env python3
"""
SODA LLM Inventory Viewer (Visualizador SSOT do SQLite)
======================================================
Este script atua exclusivamente como um extrator/visualizador de leitura (ETL Phase 3)
do banco `.souls_data/souls_heuristic_vault.db` (tabela `model_registry`).

Objetivo: Gerar um dossiê limpo, visual e cínico para o Arquiteto Humano
decidir quais modelos manter ou deletar do SSD após a avaliação do Tier 1 / Tier 2,
com separação clara entre LLMs Principais e Módulos Auxiliares (Visão mmproj / MTP).
"""

import os
import sys
import sqlite3
from pathlib import Path

def resolve_db_path() -> Path:
    """Localiza o banco de dados SSOT .souls_data/souls_heuristic_vault.db."""
    script_dir = Path(__file__).resolve().parent
    repo_root = script_dir.parent.parent
    db_path = repo_root / ".souls_data" / "souls_heuristic_vault.db"
    
    if not db_path.exists():
        cwd_db = Path.cwd() / ".souls_data" / "souls_heuristic_vault.db"
        if cwd_db.exists():
            return cwd_db
        print(f"[!] ERRO BARE-METAL: Banco SQLite não encontrado em {db_path}", file=sys.stderr)
        sys.exit(1)
        
    return db_path

def audit_schema(conn: sqlite3.Connection):
    """Extrai dinamicamente as colunas presentes na tabela model_registry."""
    cursor = conn.cursor()
    cursor.execute("PRAGMA table_info(model_registry)")
    columns_info = cursor.fetchall()
    column_names = [col[1] for col in columns_info]
    return column_names

def format_bytes(bytes_val: int) -> str:
    if not bytes_val or bytes_val <= 0:
        return "N/A"
    gb = bytes_val / (1024 ** 3)
    if gb >= 1.0:
        return f"{gb:.2f} GB"
    mb = bytes_val / (1024 ** 2)
    return f"{mb:.2f} MB"

def infer_module_type_fallback(filepath: str) -> str:
    lower = str(filepath).lower()
    if "mmproj" in lower:
        return "VISION_PROJECTOR"
    elif "mtp" in lower:
        return "MTP_ADAPTER"
    elif "bitnet" in lower:
        return "SPECIALIZED_QUANT"
    return "PRIMARY_LLM"

def generate_inventory_report():
    db_path = resolve_db_path()
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    
    cols = audit_schema(conn)
    print(f"[+] Auditando Schema da tabela 'model_registry' em {db_path.name}...")
    print(f"[+] Colunas encontradas ({len(cols)}): {', '.join(cols)}")
    
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM model_registry WHERE is_active = 1")
    all_rows = [dict(r) for r in cursor.fetchall()]
    
    # Categorização de modelos principais vs sidecars
    primary_models = []
    sidecar_modules = []
    
    for r in all_rows:
        mod_type = r.get("module_type") or infer_module_type_fallback(r.get("file_path", ""))
        r["inferred_type"] = mod_type
        if mod_type == "PRIMARY_LLM":
            primary_models.append(r)
        else:
            sidecar_modules.append(r)

    # Ordenação dos modelos principais por performance
    primary_models.sort(
        key=lambda x: (
            x.get("tier1_passed") or 0,
            x.get("success_rate_ema") or 0.0,
            -(x.get("ema_latency_ms") or 999999)
        ),
        reverse=True
    )
    
    # Mapeamento de Sidecars para cada Modelo Principal (por diretório/família)
    for p in primary_models:
        p_dir = os.path.dirname(p.get("file_path", ""))
        attached = []
        for s in sidecar_modules:
            s_dir = os.path.dirname(s.get("file_path", ""))
            s_name = os.path.basename(s.get("file_path", ""))
            if p_dir == s_dir or p.get("family", "").lower() in s_name.lower():
                attached.append((s.get("inferred_type"), s_name, format_bytes(s.get("file_size_bytes", 0))))
        p["attached_modules"] = attached

    # Caminho do relatório de auditoria TXT
    repo_root = db_path.parent.parent
    audit_dir = repo_root / "docs" / "audits" / "local_llms"
    audit_dir.mkdir(parents=True, exist_ok=True)
    report_file = audit_dir / "soda_llms_inventory_dossier.txt"
    
    lines = []
    lines.append("================================================================================")
    lines.append("                DOSSIÊ DE INVENTÁRIO LLM - SODA SSOT BARE-METAL                 ")
    lines.append("================================================================================")
    lines.append(f"Banco de Dados SSOT: {db_path}")
    lines.append(f"Total de Arquivos GGUF no SSD: {len(all_rows)}")
    lines.append(f"LLMs Principais de Texto: {len(primary_models)}")
    lines.append(f"Módulos Auxiliares (Visão/MTP/Sidecars): {len(sidecar_modules)}")
    lines.append("================================================================================")
    lines.append("")
    
    lines.append("================================================================================")
    lines.append("                       PARTE 1: LLMs PRINCIPAIS DE TEXTO                        ")
    lines.append("================================================================================")
    lines.append("")
    
    if not primary_models:
        lines.append("  [!] Nenhuma LLM principal registrada no banco de dados SQLite.")
        lines.append("")
    else:
        for idx, row_dict in enumerate(primary_models, start=1):
            model_id = row_dict.get("file_path") or f"Modelo #{idx}"
            name = row_dict.get("model_name") or os.path.basename(str(model_id))
            family = row_dict.get("family") or "Desconhecida"
            params = row_dict.get("parameters") or "N/A"
            ctx = row_dict.get("context_length") or "N/A"
            quant = row_dict.get("quantization") or "N/A"
            caps = row_dict.get("capabilities") or "[]"
            size_b = row_dict.get("file_size_bytes") or 0
            size_str = format_bytes(size_b)
            
            lat = row_dict.get("ema_latency_ms", 0.0) or 0.0
            lat_str = f"{lat:.2f} ms" if isinstance(lat, (int, float)) and lat > 0 else "N/A (Não medido)"
            
            succ = row_dict.get("success_rate_ema", 0.0) or 0.0
            succ_pct = (succ * 100.0) if (isinstance(succ, (int, float)) and succ <= 1.0) else succ
            succ_str = f"{succ_pct:.1f}%" if (isinstance(succ_pct, (int, float)) and (succ_pct > 0 or lat > 0)) else "N/A"

            last_seen = row_dict.get("last_seen", "N/A")
            t1_val = row_dict.get("tier1_passed")

            if t1_val == 1:
                tier1_status = "Aprovado (Tier 1 Passed)"
                cynical_note = "RETENÇÃO RECOMENDADA: Modelo de alta performance e sintaxe estável (Aprovado no Tier 1)."
            elif (lat > 0 or succ > 0) or (t1_val == 0 and isinstance(succ_pct, (int, float)) and succ_pct > 0):
                tier1_status = "Reprovado (Guilhotina)"
                cynical_note = "CANDIDATO À PURGA DO SSD: Falhas sintáticas recorrentes ou reprovação na Guilhotina Tier 1."
            else:
                tier1_status = "Pendente (Não Testado)"
                cynical_note = "AGUARDANDO AVALIAÇÃO: Modelo ainda não submetido à suíte de benchmarking Tier 1."

            attached_str = "Nenhum"
            if row_dict["attached_modules"]:
                attached_str = ", ".join([f"[{mtype}: {mname} ({msize})]" for mtype, mname, msize in row_dict["attached_modules"]])

            lines.append("==================================================")
            lines.append(f"LLM #{idx}: {name}")
            lines.append(f"> ID / Path: {model_id}")
            lines.append(f"> Status Tier 1: {tier1_status}")
            lines.append(f"> Performance: Latência: {lat_str} | Sucesso Sintático: {succ_str}")
            lines.append(f"> Metadados Físicos: Família: {family} | Parâmetros: {params} | Contexto Máximo: {ctx} | Quantização: {quant}")
            lines.append(f"> Tamanho / VRAM: {size_str} | Capacidades: {caps} | Visto em: {last_seen}")
            lines.append(f"> Módulos Anexados: {attached_str}")
            lines.append(f"> Veredito do SODA: {cynical_note}")
            lines.append("==================================================")
            lines.append("")
            
    lines.append("================================================================================")
    lines.append("              PARTE 2: MÓDULOS AUXILIARES E SIDECARS (VISÃO / MTP)             ")
    lines.append("================================================================================")
    lines.append("")
    
    if not sidecar_modules:
        lines.append("  [i] Nenhum módulo auxiliar ou sidecar encontrado.")
    else:
        for idx, s in enumerate(sidecar_modules, start=1):
            s_path = s.get("file_path", "")
            s_name = os.path.basename(s_path)
            s_type = s.get("inferred_type", "SIDECAR")
            s_size = format_bytes(s.get("file_size_bytes", 0))
            
            lines.append(f"MÓDULO #{idx}: {s_name}")
            lines.append(f"> Tipo: {s_type} | Tamanho: {s_size}")
            lines.append(f"> Path: {s_path}")
            lines.append(f"> Função: {'Encoder de Visão (Projetor de Imagens)' if s_type == 'VISION_PROJECTOR' else 'Adaptador de Especulação MTP' if s_type == 'MTP_ADAPTER' else 'Quantização Especializada'}")
            lines.append("-" * 50)
            lines.append("")

    lines.append("================================================================================")
    lines.append(" FIM DO DOSSIÊ - GERADO AUTOMATICAMENTE PELO VISUALIZADOR DE INVENTÁRIO SODA  ")
    lines.append("================================================================================")
    
    with open(report_file, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
        
    print(f"[+] Dossiê de inventário gerado com sucesso em:")
    print(f"    {report_file}")
    
    # Exibe resumo no console para rápida visualização
    app_count = sum(1 for p in primary_models if p.get("tier1_passed") == 1)
    rep_count = sum(1 for p in primary_models if p.get("tier1_passed") == 0 and (p.get("ema_latency_ms") or 0) > 0)
    pen_count = sum(1 for p in primary_models if (p.get("ema_latency_ms") or 0) == 0 and (p.get("success_rate_ema") or 0) == 0)

    print(f"\n================================================================================")
    print(f"       RESUMO EXECUTIVO DO INVENTÁRIO LLM SODA ({len(primary_models)} LLMs Texto + {len(sidecar_modules)} Sidecars)      ")
    print(f"================================================================================")
    print(f"Arquivos GGUF no SSD: {len(all_rows)} | LLMs Principais: {len(primary_models)} (Aprovadas: {app_count} | Reprovadas: {rep_count} | Pendentes: {pen_count})")
    print(f"Módulos Auxiliares: Visão (mmproj): {sum(1 for s in sidecar_modules if s['inferred_type'] == 'VISION_PROJECTOR')} | MTP: {sum(1 for s in sidecar_modules if s['inferred_type'] == 'MTP_ADAPTER')} | Outros: {sum(1 for s in sidecar_modules if s['inferred_type'] == 'SPECIALIZED_QUANT')}")
    print(f"================================================================================")
    print(f"{'#':<3} | {'NOME DO MODELO':<30} | {'STATUS TIER 1':<22} | {'SUCESSO':<8} | {'MÓDULOS ANEXADOS':<18}")
    print(f"-" * 90)
    for idx, rd in enumerate(primary_models, start=1):
        m_name = (rd.get("model_name") or os.path.basename(str(rd.get("file_path", ""))))[:30]
        t1 = rd.get("tier1_passed")
        lat = rd.get("ema_latency_ms", 0.0) or 0.0
        succ = rd.get("success_rate_ema", 0.0) or 0.0
        succ_pct = (succ * 100.0) if (isinstance(succ, (int, float)) and succ <= 1.0) else succ

        if t1 == 1:
            st = "APROVADO TIER 2"
        elif lat > 0 or succ > 0:
            st = "REPROVADO (GUILHOTINA)"
        else:
            st = "PENDENTE"

        s_str = f"{succ_pct:.1f}%" if (lat > 0 or succ > 0) else "N/A"
        att_str = "VISÃO" if any(m[0] == "VISION_PROJECTOR" for m in rd["attached_modules"]) else "Nenhum"
        print(f"{idx:<3} | {m_name:<30} | {st:<22} | {s_str:<8} | {att_str:<18}")
    print(f"================================================================\n")

    conn.close()

if __name__ == "__main__":
    generate_inventory_report()
