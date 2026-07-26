#!/usr/bin/env python3
"""
SODA LLM Inventory Viewer (Visualizador SSOT do SQLite)
======================================================
Este script atua exclusivamente como um extrator/visualizador de leitura (ETL Phase 3)
do banco `.soda_data/soda_heuristic_vault.db` (tabela `model_registry`).

Objetivo: Gerar um dossiê limpo, visual e cínico para o Arquiteto Humano
decidir quais modelos manter ou deletar do SSD após a avaliação do Tier 1 / Tier 2.
"""

import os
import sys
import sqlite3
from pathlib import Path

def resolve_db_path() -> Path:
    """Localiza o banco de dados SSOT .soda_data/soda_heuristic_vault.db."""
    script_dir = Path(__file__).resolve().parent
    repo_root = script_dir.parent.parent
    db_path = repo_root / ".soda_data" / "soda_heuristic_vault.db"
    
    if not db_path.exists():
        cwd_db = Path.cwd() / ".soda_data" / "soda_heuristic_vault.db"
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

def generate_inventory_report():
    db_path = resolve_db_path()
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    
    cols = audit_schema(conn)
    print(f"[+] Auditando Schema da tabela 'model_registry' em {db_path.name}...")
    print(f"[+] Colunas encontradas ({len(cols)}): {', '.join(cols)}")
    
    # Monta cláusula ORDER BY com base nas colunas disponíveis
    order_by_parts = []
    if "tier1_passed" in cols:
        order_by_parts.append("tier1_passed DESC")
    if "success_rate_ema" in cols:
        order_by_parts.append("success_rate_ema DESC")
    if "ema_latency_ms" in cols:
        order_by_parts.append("ema_latency_ms ASC")
        
    order_clause = "ORDER BY " + ", ".join(order_by_parts) if order_by_parts else ""
    
    # Query de seleção apenas dos modelos ativos (is_active = 1)
    if "is_active" in cols:
        query = f"SELECT * FROM model_registry WHERE is_active = 1 {order_clause}"
    else:
        query = f"SELECT * FROM model_registry {order_clause}"
        
    cursor = conn.cursor()
    cursor.execute(query)
    rows = cursor.fetchall()
    
    # Caminho do relatório de auditoria TXT
    repo_root = db_path.parent.parent
    audit_dir = repo_root / "docs" / "audits" / "local_llms"
    audit_dir.mkdir(parents=True, exist_ok=True)
    report_file = audit_dir / "soda_llms_inventory_dossier.txt"
    
    lines = []
    lines.append("================================================================================")
    lines.append("                DOSSIÊ DE INVENTÁRIO LLM - SODA SSOT BARE-METAL                 ")
    lines.append("================================================================================")
    lines.append(f"Banco de Dados: {db_path}")
    lines.append(f"Total de Modelos Ativos Encontrados: {len(rows)}")
    lines.append("================================================================================")
    lines.append("")
    
    if not rows:
        lines.append("  [!] Nenhum modelo ativo registrado no banco de dados SQLite.")
        lines.append("  Rode a suíte de benchmarking Tier 1 para popular o model_registry.")
        lines.append("")
    else:
        for idx, r in enumerate(rows, start=1):
            row_dict = dict(r)
            
            # Identificação do Modelo
            model_id = row_dict.get("file_path") or row_dict.get("model_id") or row_dict.get("model_name") or f"Modelo #{idx}"
            name = row_dict.get("model_name") or os.path.basename(str(model_id))
            family = row_dict.get("family") or row_dict.get("provider_type") or "Desconhecida"
            params = row_dict.get("parameters") or "N/A"
            ctx = row_dict.get("context_length") or row_dict.get("max_context_window") or "N/A"
            quant = row_dict.get("quantization") or "N/A"
            caps = row_dict.get("capabilities") or row_dict.get("specialty_tags") or "[]"
            size_b = row_dict.get("file_size_bytes") or 0
            size_str = format_bytes(size_b) if size_b else f"VRAM Base: {row_dict.get('vram_base_mb', 'N/A')} MB"
            
            # Status Tier 1
            t1_val = row_dict.get("tier1_passed")
            if t1_val is None:
                tier1_status = "Pendente"
            elif t1_val == 1:
                tier1_status = "Aprovado (Tier 1 Passed)"
            else:
                tier1_status = "Reprovado (Guilhotina)"
                
            # Métricas de Desempenho
            lat = row_dict.get("ema_latency_ms", 0.0)
            lat_str = f"{lat:.2f} ms" if isinstance(lat, (int, float)) else f"{lat} ms"
            
            succ = row_dict.get("success_rate_ema", 0.0)
            succ_pct = (succ * 100.0) if (isinstance(succ, (int, float)) and succ <= 1.0) else succ
            succ_str = f"{succ_pct:.1f}%" if isinstance(succ_pct, (int, float)) else f"{succ_pct}%"
            
            # Diagnóstico Cínico para Decisão de Purga
            if t1_val == 1 and (isinstance(succ_pct, (int, float)) and succ_pct >= 80.0):
                cynical_note = "RETENÇÃO RECOMENDADA: Modelo de alta performance e sintaxe estável."
            elif t1_val == 0 or (isinstance(succ_pct, (int, float)) and succ_pct < 50.0 and succ_pct > 0):
                cynical_note = "CANDIDATO À PURGA DO SSD: Falhas sintáticas recorrentes ou reprovação no Tier 1."
            else:
                cynical_note = "AGUARDANDO AVALIAÇÃO: Dados insuficientes para veredito de eliminação."

            lines.append("==================================================")
            lines.append(f"MODELO #{idx}: {name}")
            lines.append(f"> ID / Path: {model_id}")
            lines.append(f"> Status Tier 1: {tier1_status}")
            lines.append(f"> Performance: Latência: {lat_str} | Sucesso Sintático: {succ_str}")
            lines.append(f"> Metadados Físicos: Família: {family} | Parâmetros: {params} | Contexto Máximo: {ctx} | Quantização: {quant}")
            lines.append(f"> Tamanho / VRAM: {size_str} | Capacidades: {caps}")
            lines.append(f"> Veredito do SODA: {cynical_note}")
            lines.append("==================================================")
            lines.append("")
            
    lines.append("================================================================================")
    lines.append(" FIM DO DOSSIÊ - GERADO AUTOMATICAMENTE PELO VISUALIZADOR DE INVENTÁRIO SODA  ")
    lines.append("================================================================================")
    
    with open(report_file, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
        
    print(f"[+] Dossiê de inventário gerado com sucesso em:")
    print(f"    {report_file}")
    conn.close()

if __name__ == "__main__":
    generate_inventory_report()
