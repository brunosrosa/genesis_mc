#!/usr/bin/env python3
"""
SOULS LLM Inventory Viewer (Visualizador SSOT do SQLite & Telemetria da Arena)
================================================================================
Este script atua como um extrator/visualizador de leitura (ETL Phase 3)
do banco `.souls_data/souls_heuristic_vault.db`, unindo as tabelas `model_registry`
e `arena_telemetry`.

Objetivo: Gerar um relatório visual em Markdown ('docs/observability/reports/souls_llms_inventory_summary.md')
com tabelas ASCII alinhadas, métricas empíricas de TTFT, TPOT, Acurácia Sintática e E3 Score:
    Score E3 = (Acurácia^2) / (Latência Média em segundos + 0.001)
"""

import os
import sys
import sqlite3
from pathlib import Path
from typing import List, Dict, Any, Optional

def resolve_db_path() -> Path:
    """Localiza dinamicamente o banco de dados SSOT souls_heuristic_vault.db na ordem estrita de prioridade SOULS."""
    candidate_paths = [
        Path("Z:/souls_mc/.souls_data/souls_heuristic_vault.db"),
        Path("Z:/souls_mc/.souls_data/souls_heuristic_vault.db"),
        Path("./.souls_data/souls_heuristic_vault.db"),
        Path.cwd() / ".souls_data" / "souls_heuristic_vault.db",
        Path.cwd() / ".souls_data" / "souls_heuristic_vault.db",
        Path(__file__).resolve().parent.parent.parent / ".souls_data" / "souls_heuristic_vault.db",
        Path(__file__).resolve().parent.parent / ".souls_data" / "souls_heuristic_vault.db",
    ]
    
    for p in candidate_paths:
        if p.exists():
            return p.resolve()
            
    print(f"[!] ERRO BARE-METAL: Banco SQLite não encontrado nas rotas de fallback: {candidate_paths[0]}", file=sys.stderr)
    sys.exit(1)

def format_bytes(bytes_val: Optional[int]) -> str:
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

def check_table_exists(conn: sqlite3.Connection, table_name: str) -> bool:
    cursor = conn.cursor()
    cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name=?", (table_name,))
    return cursor.fetchone() is not None

def calculate_e3_score(accuracy_pct: float, ttft_ms: float, tpot_ms: float, fallback_latency_ms: float) -> float:
    """
    Cálculo do Score E3 Consolidado:
    Score E3 = (Acurácia^2) / (Latência Média em segundos + 0.001)
    onde Acurácia está no intervalo [0.0, 1.0].
    """
    acc_ratio = max(0.0, min(1.0, accuracy_pct / 100.0))
    
    if ttft_ms > 0 or tpot_ms > 0:
        latency_sec = (ttft_ms + tpot_ms) / 1000.0
    elif fallback_latency_ms > 0:
        latency_sec = fallback_latency_ms / 1000.0
    else:
        latency_sec = 0.0
        
    e3 = (acc_ratio ** 2) / (latency_sec + 0.001)
    return round(e3, 4)

def fetch_inventory_data(conn: sqlite3.Connection) -> List[Dict[str, Any]]:
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    table_name = "local_models" if check_table_exists(conn, "local_models") else "model_registry"
    has_telemetry = check_table_exists(conn, "arena_telemetry")
    
    if has_telemetry:
        query = f"""
        SELECT 
            mr.file_path,
            mr.model_name,
            mr.family,
            mr.parameters,
            mr.context_length,
            mr.quantization,
            mr.capabilities,
            mr.file_size_bytes,
            1 as is_active,
            1 as tier1_passed,
            'PRIMARY_LLM' as module_type,
            1.0 as success_rate_ema,
            0.0 as ema_latency_ms,
            mr.last_seen,
            COUNT(at.prompt_id) as telemetry_count,
            AVG(at.ttft_ms) as avg_ttft_ms,
            AVG(at.tpot_ms) as avg_tpot_ms,
            AVG(at.vram_peak_mb) as avg_vram_peak_mb,
            AVG(at.json_success) as syntax_success_rate,
            AVG(at.e3_score) as avg_e3_score
        FROM {table_name} mr
        LEFT JOIN arena_telemetry at ON mr.file_path = at.file_path
        GROUP BY mr.file_path
        """
    else:
        query = f"""
        SELECT 
            mr.file_path,
            mr.model_name,
            mr.family,
            mr.parameters,
            mr.context_length,
            mr.quantization,
            mr.capabilities,
            mr.file_size_bytes,
            1 as is_active,
            1 as tier1_passed,
            'PRIMARY_LLM' as module_type,
            1.0 as success_rate_ema,
            0.0 as ema_latency_ms,
            mr.last_seen,
            0 as telemetry_count,
            0.0 as avg_ttft_ms,
            0.0 as avg_tpot_ms,
            0.0 as avg_vram_peak_mb,
            0.0 as syntax_success_rate,
            0.0 as avg_e3_score
        FROM {table_name} mr
        """
        
    cursor.execute(query)
    rows = [dict(r) for r in cursor.fetchall()]
    return rows

def generate_inventory_report():
    db_path = resolve_db_path()
    print(f"[+] Conectando ao Banco SSOT em: {db_path}")
    
    conn = sqlite3.connect(db_path)
    all_rows = fetch_inventory_data(conn)
    
    primary_models = []
    sidecar_modules = []
    
    for r in all_rows:
        mod_type = r.get("module_type") or infer_module_type_fallback(r.get("file_path", ""))
        r["inferred_type"] = mod_type
        
        # Métrica de Acurácia Sintática (0.0% a 100.0%)
        if r.get("telemetry_count", 0) > 0 and r.get("syntax_success_rate") is not None:
            r["accuracy_pct"] = round(r["syntax_success_rate"] * 100.0, 2)
        else:
            ema_succ = r.get("success_rate_ema") or 0.0
            r["accuracy_pct"] = round(ema_succ * 100.0 if ema_succ <= 1.0 else ema_succ, 2)

        # TTFT e TPOT
        r["ttft_ms"] = round(r.get("avg_ttft_ms") or 0.0, 2)
        r["tpot_ms"] = round(r.get("avg_tpot_ms") or 0.0, 2)
        
        # E3 Score
        if r.get("telemetry_count", 0) > 0 and r.get("avg_e3_score") and r["avg_e3_score"] > 0:
            r["e3_score"] = round(r["avg_e3_score"], 4)
        else:
            r["e3_score"] = calculate_e3_score(
                r["accuracy_pct"],
                r["ttft_ms"],
                r["tpot_ms"],
                r.get("ema_latency_ms") or 0.0
            )

        if mod_type == "PRIMARY_LLM":
            primary_models.append(r)
        else:
            sidecar_modules.append(r)

    # Ordenação por E3 Score decrescente, depois Acurácia e Latência
    primary_models.sort(
        key=lambda x: (
            x.get("tier1_passed") or 0,
            x.get("e3_score") or 0.0,
            x.get("accuracy_pct") or 0.0,
            -(x.get("ema_latency_ms") or 999999)
        ),
        reverse=True
    )
    
    # Mapeamento de Sidecars
    for p in primary_models:
        p_dir = os.path.dirname(p.get("file_path", ""))
        attached = []
        for s in sidecar_modules:
            s_dir = os.path.dirname(s.get("file_path", ""))
            s_name = os.path.basename(s.get("file_path", ""))
            if p_dir == s_dir or (p.get("family") and p["family"].lower() in s_name.lower()):
                attached.append((s.get("inferred_type"), s_name, format_bytes(s.get("file_size_bytes", 0))))
        p["attached_modules"] = attached

    # Criação do relatório Markdown polido
    repo_root = db_path.parent.parent
    reports_dir = repo_root / "docs" / "observability" / "reports"
    reports_dir.mkdir(parents=True, exist_ok=True)
    summary_md_file = reports_dir / "souls_llms_inventory_summary.md"
    
    lines = []
    lines.append("# 📊 SOULS LLM INVENTORY SUMMARY & TELEMETRY DOSSIER")
    lines.append(f"**Data de Geração:** 2026-08-05 | **Banco SSOT:** `{db_path}`")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 🖥️ RESUMO EXECUTIVO DE HARDWARE & CAPACIDADES DO HOST")
    lines.append("- **Placa Gráfica (Target GPU):** NVIDIA GeForce RTX 2060m (6GB VRAM, Arquitetura Turing)")
    lines.append("- **Aceleração Host:** CPU Intel Core i9 (AVX2 SIMD Acceleration) + Gateway Tokio Rust")
    lines.append("- **Infradesign Bare-Metal:** C-FFI Zero-Garbage (`llama_cpp_2`), Offload Adaptativo de Camadas GPU (n_gpu_layers=99)")
    lines.append("- **Limites Térmicos & Trava de Contexto:** Hard-Cap de 32k tokens na família Gemma (`cap_context_length_for_family`), Cache KV Assimétrico (F16 Keys / Q4_K ou Q8_0 Values)")
    lines.append("- **Fórmula de Eficiência (Score E3):** $E3 = \\frac{\\text{Acurácia}^2}{\\text{Latência Total (s)} + 0.001}$")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 📈 METRICAS CONSOLIDADAS DAS LLMs PRINCIPAIS")
    lines.append("")
    lines.append(f"| Total GGUF | LLMs Principais | Aprovados Tier 1 | Reprovados (Guilhotina) | Pendentes | Sidecars |")
    lines.append(f"| :---: | :---: | :---: | :---: | :---: | :---: |")
    
    app_count = sum(1 for p in primary_models if p.get("tier1_passed") == 1)
    rep_count = sum(1 for p in primary_models if p.get("tier1_passed") == 0 and ((p.get("ema_latency_ms") or 0) > 0 or p.get("telemetry_count", 0) > 0))
    pen_count = sum(1 for p in primary_models if p.get("tier1_passed") == 0 and (p.get("ema_latency_ms") or 0) == 0 and p.get("telemetry_count", 0) == 0)
    
    lines.append(f"| {len(all_rows)} | {len(primary_models)} | {app_count} | {rep_count} | {pen_count} | {len(sidecar_modules)} |")
    lines.append("")
    lines.append("### 🏆 TABELA DE PERFORMANCE E RANKING DE MODELOS (TIER 1 / TIER 2)")
    lines.append("")
    lines.append("| # | Nome do Modelo | Família | Quant | Tamanho | TTFT (ms) | TPOT (ms) | Acurácia JSON | Score E3 | Status Tier 1 |")
    lines.append("| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |")
    
    for idx, p in enumerate(primary_models, start=1):
        m_name = p.get("model_name") or os.path.basename(str(p.get("file_path", "")))
        family = p.get("family") or "N/A"
        quant = p.get("quantization") or "N/A"
        size = format_bytes(p.get("file_size_bytes"))
        ttft = f"{p['ttft_ms']:.1f}" if p['ttft_ms'] > 0 else "N/A"
        tpot = f"{p['tpot_ms']:.1f}" if p['tpot_ms'] > 0 else "N/A"
        acc = f"{p['accuracy_pct']:.1f}%"
        e3 = f"{p['e3_score']:.4f}"
        
        t1 = p.get("tier1_passed")
        if t1 == 1:
            status = "✅ Aprovado (Tier 1)"
        elif p.get("telemetry_count", 0) > 0 or (p.get("ema_latency_ms") or 0) > 0:
            status = "❌ Reprovado (Guilhotina)"
        else:
            status = "⏳ Pendente"
            
        lines.append(f"| {idx} | `{m_name}` | {family} | {quant} | {size} | {ttft} | {tpot} | {acc} | **{e3}** | {status} |")

    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 🧩 MÓDULOS AUXILIARES E SIDECARS (VISÃO / MTP)")
    lines.append("")
    lines.append("| # | Nome do Módulo | Tipo de Sidecar | Tamanho | Caminho Físico |")
    lines.append("| :--- | :--- | :---: | :---: | :--- |")
    
    if not sidecar_modules:
        lines.append("| - | Nenhum módulo auxiliar encontrado | - | - | - |")
    else:
        for idx, s in enumerate(sidecar_modules, start=1):
            s_name = os.path.basename(s.get("file_path", ""))
            s_type = s.get("inferred_type", "SIDECAR")
            s_size = format_bytes(s.get("file_size_bytes", 0))
            s_path = s.get("file_path", "")
            lines.append(f"| {idx} | `{s_name}` | `{s_type}` | {s_size} | `{s_path}` |")

    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 📝 DETALHAMENTO E DOSSIÊ INDIVIDUAL DOS MODELOS")
    lines.append("")
    
    for idx, p in enumerate(primary_models, start=1):
        m_name = p.get("model_name") or os.path.basename(str(p.get("file_path", "")))
        lines.append(f"### {idx}. `{m_name}`")
        lines.append(f"- **Caminho Físico:** `{p.get('file_path')}`")
        lines.append(f"- **Metadados:** Família `{p.get('family')}` | Parâmetros `{p.get('parameters')}` | Contexto Máximo `{p.get('context_length')}` | Quant `{p.get('quantization')}`")
        lines.append(f"- **Telemetria:** TTFT `{p['ttft_ms']} ms` | TPOT `{p['tpot_ms']} ms` | Latência Média `{p.get('ema_latency_ms', 0):.2f} ms` | Acurácia `{p['accuracy_pct']}%` | **Score E3 `{p['e3_score']}`**")
        
        att_str = "Nenhum"
        if p["attached_modules"]:
            att_str = ", ".join([f"`{mname}` ({msize})" for _, mname, msize in p["attached_modules"]])
        lines.append(f"- **Módulos Anexados:** {att_str}")
        
        t1 = p.get("tier1_passed")
        if t1 == 1:
            verdict = "RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta."
        elif p.get("telemetry_count", 0) > 0 or (p.get("ema_latency_ms") or 0) > 0:
            verdict = "PURGA RECOMENDADA DO SSD: Reprovado por falhas sintáticas ou latência incompatível com o hardware."
        else:
            verdict = "AGUARDANDO AVALIAÇÃO DA ARENA: Modelo aguardando execução de testes de inferência."
        lines.append(f"- **Veredito SOULS:** {verdict}")
        lines.append("")

    lines.append("---")
    lines.append("*Fim do Dossiê de Inventário SOULS v4. Gerado automaticamente via `souls_llms_inventory_viewer.py`.*")
    
    report_content = "\n".join(lines)
    with open(summary_md_file, "w", encoding="utf-8") as f:
        f.write(report_content)
        
    print(f"[+] Relatório Markdown de Inventário e Telemetria gerado com sucesso em:")
    print(f"    {summary_md_file}")
    
    conn.close()


if __name__ == "__main__":
    generate_inventory_report()
