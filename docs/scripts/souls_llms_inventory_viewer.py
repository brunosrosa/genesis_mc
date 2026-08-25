#!/usr/bin/env python3
"""
SOULS LLM Inventory Viewer & Arena Telemetry Dossier (Fase 1.5 Consciência do Silício)
======================================================================================
Extrator e visualizador SSOT do banco `.souls_data/souls_heuristic_vault.db` e telemetria
empírica gerada pelo `souls_arena_cli`.

Gera relatório visual em Markdown ('docs/observability/reports/souls_llms_inventory_summary.md')
com métricas de TTFT, TPOT, Acurácia Sintática, E3 Score e Scores Cognitivos Especializados:
    Score E3 = (Acurácia^2) / (Latência Média em segundos + 0.001)
"""

import os
import sys
import glob
import sqlite3
from pathlib import Path
from typing import List, Dict, Any, Optional
from datetime import datetime

def resolve_db_path() -> Path:
    """Localiza dinamicamente o banco de dados SSOT souls_heuristic_vault.db na ordem estrita de prioridade SOULS."""
    candidate_paths = [
        Path(__file__).resolve().parent.parent.parent / ".souls_data" / "souls_heuristic_vault.db",
        Path.cwd() / ".souls_data" / "souls_heuristic_vault.db",
        Path("Z:/souls_mc/.souls_data/souls_heuristic_vault.db"),
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
    if "mmproj" in lower or "clip" in lower:
        return "VISION_PROJECTOR"
    elif "mtp" in lower:
        return "MTP_ADAPTER"
    elif "dspark" in lower or "draft" in lower:
        return "SPECULATIVE_DRAFT"
    elif "bitnet" in lower or "i2_s" in lower or "i1_s" in lower:
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
    
    table_name = "model_registry" if check_table_exists(conn, "model_registry") else "local_models"
    has_telemetry = check_table_exists(conn, "arena_telemetry")
    
    # Inspeciona colunas disponíveis
    cursor.execute(f"PRAGMA table_info({table_name})")
    cols = {row["name"] for row in cursor.fetchall()}
    
    col_ttft = "mr.ttft_ms" if "ttft_ms" in cols else "0.0 as ttft_ms"
    col_tpot = "mr.tpot_ms" if "tpot_ms" in cols else "0.0 as tpot_ms"
    col_vram = "mr.vram_peak_mb" if "vram_peak_mb" in cols else "0.0 as vram_peak_mb"
    col_e3 = "mr.e3_score" if "e3_score" in cols else "0.0 as e3_score"
    col_json = "mr.score_json_tools" if "score_json_tools" in cols else "0.0 as score_json_tools"
    col_code = "mr.score_code_ast" if "score_code_ast" in cols else "0.0 as score_code_ast"
    col_reason = "mr.score_reasoning" if "score_reasoning" in cols else "0.0 as score_reasoning"
    col_vision = "mr.score_vision_vqa" if "score_vision_vqa" in cols else "0.0 as score_vision_vqa"
    col_has_mmproj = "mr.has_mmproj_sidecar" if "has_mmproj_sidecar" in cols else "0 as has_mmproj_sidecar"
    col_mmproj_path = "mr.mmproj_file_path" if "mmproj_file_path" in cols else "NULL as mmproj_file_path"
    col_module_type = "mr.module_type" if "module_type" in cols else "'PRIMARY_LLM' as module_type"

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
            mr.is_active,
            mr.tier1_passed,
            {col_module_type},
            mr.success_rate_ema,
            mr.ema_latency_ms,
            {col_ttft},
            {col_tpot},
            {col_vram},
            {col_e3},
            {col_json},
            {col_code},
            {col_reason},
            {col_vision},
            {col_has_mmproj},
            {col_mmproj_path},
            mr.last_seen,
            COUNT(at.prompt_id) as telemetry_count,
            AVG(at.ttft_ms) as dyn_ttft_ms,
            AVG(at.tpot_ms) as dyn_tpot_ms,
            AVG(at.vram_peak_mb) as dyn_vram_peak_mb,
            AVG(at.json_success) as syntax_success_rate,
            AVG(at.e3_score) as dyn_e3_score
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
            mr.is_active,
            mr.tier1_passed,
            {col_module_type},
            mr.success_rate_ema,
            mr.ema_latency_ms,
            {col_ttft},
            {col_tpot},
            {col_vram},
            {col_e3},
            {col_json},
            {col_code},
            {col_reason},
            {col_vision},
            {col_has_mmproj},
            {col_mmproj_path},
            mr.last_seen,
            0 as telemetry_count,
            0.0 as dyn_ttft_ms,
            0.0 as dyn_tpot_ms,
            0.0 as dyn_vram_peak_mb,
            0.0 as syntax_success_rate,
            0.0 as dyn_e3_score
        FROM {table_name} mr
        """
        
    cursor.execute(query)
    rows = [dict(r) for r in cursor.fetchall()]
    return rows

def scan_embedded_core_models(repo_root: Path) -> List[Dict[str, Any]]:
    """Varre modelos e tokenizers internos em src-tauri/models."""
    core_models_dir = repo_root / "src-tauri" / "models"
    items = []
    if core_models_dir.exists():
        for p in core_models_dir.glob("*"):
            if p.is_file():
                ext = p.suffix.lower()
                sz = p.stat().st_size
                desc = "Modelo ONNX de Classificação de Intenções / NER" if "gliclass" in p.name.lower() or ext == ".onnx" else "Arquivo de Configuração / Tokenizer Core"
                if ext in [".onnx", ".data", ".safetensors", ".bin", ".gguf"]:
                    cat = "MODEL_WEIGHTS"
                elif ext == ".json":
                    cat = "TOKENIZER_CONFIG"
                else:
                    cat = "AUXILIARY"
                items.append({
                    "file_name": p.name,
                    "file_path": str(p),
                    "file_size_bytes": sz,
                    "category": cat,
                    "description": desc,
                    "extension": ext.upper().replace(".", "")
                })
    items.sort(key=lambda x: x["file_size_bytes"], reverse=True)
    return items

def generate_inventory_report():
    db_path = resolve_db_path()
    repo_root = db_path.parent.parent
    print(f"[+] Conectando ao Banco SSOT em: {db_path}")
    
    conn = sqlite3.connect(db_path)
    all_rows = fetch_inventory_data(conn)
    conn.close()

    embedded_models = scan_embedded_core_models(repo_root)
    
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

        # TTFT e TPOT (Prioriza telemetria dinâmica ou métrica da model_registry)
        ttft = r.get("dyn_ttft_ms") if (r.get("dyn_ttft_ms") or 0.0) > 0 else r.get("ttft_ms", 0.0)
        tpot = r.get("dyn_tpot_ms") if (r.get("dyn_tpot_ms") or 0.0) > 0 else r.get("tpot_ms", 0.0)
        r["ttft_ms"] = round(ttft or 0.0, 2)
        r["tpot_ms"] = round(tpot or 0.0, 2)
        
        # E3 Score
        e3_db = r.get("dyn_e3_score") if (r.get("dyn_e3_score") or 0.0) > 0 else r.get("e3_score", 0.0)
        if e3_db and e3_db > 0:
            r["e3_score"] = round(e3_db, 4)
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
        # mmproj vinculado no SQLite
        if p.get("has_mmproj_sidecar") == 1 and p.get("mmproj_file_path"):
            attached.append(("VISION_PROJECTOR", os.path.basename(p["mmproj_file_path"]), "Pareado SQLite"))
        for s in sidecar_modules:
            s_dir = os.path.dirname(s.get("file_path", ""))
            s_name = os.path.basename(s.get("file_path", ""))
            if (p_dir == s_dir or (p.get("family") and p["family"].lower() in s_name.lower())) and not any(a[1] == s_name for a in attached):
                attached.append((s.get("inferred_type"), s_name, format_bytes(s.get("file_size_bytes", 0))))
        p["attached_modules"] = attached

    # Criação do relatório Markdown polido
    reports_dir = repo_root / "docs" / "observability" / "reports"
    reports_dir.mkdir(parents=True, exist_ok=True)
    summary_md_file = reports_dir / "souls_llms_inventory_summary.md"
    
    lines = []
    lines.append("# 📊 SOULS LLM INVENTORY SUMMARY & TELEMETRY DOSSIER")
    lines.append(f"**Data de Geração:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')} | **Banco SSOT:** `{db_path}`")
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
    lines.append("## 📈 MÉTRICAS CONSOLIDADAS DAS LLMs PRINCIPAIS")
    lines.append("")
    lines.append(f"| Total GGUF | LLMs Principais | Aprovados Tier 1 | Reprovados (Guilhotina) | Pendentes | Sidecars | Modelos Core (src-tauri) |")
    lines.append(f"| :---: | :---: | :---: | :---: | :---: | :---: | :---: |")
    
    app_count = sum(1 for p in primary_models if p.get("tier1_passed") == 1)
    rep_count = sum(1 for p in primary_models if p.get("tier1_passed") == 0 and ((p.get("ema_latency_ms") or 0) > 0 or p.get("telemetry_count", 0) > 0 or p.get("ttft_ms", 0) > 0))
    pen_count = sum(1 for p in primary_models if p.get("tier1_passed") == 0 and (p.get("ema_latency_ms") or 0) == 0 and p.get("telemetry_count", 0) == 0 and p.get("ttft_ms", 0) == 0)
    
    lines.append(f"| {len(all_rows)} | {len(primary_models)} | {app_count} | {rep_count} | {pen_count} | {len(sidecar_modules)} | {len(embedded_models)} |")
    lines.append("")
    lines.append("### 🏆 TABELA DE PERFORMANCE E RANKING DE MODELOS (TIER 1 / TIER 2)")
    lines.append("")
    lines.append("| # | Nome do Modelo | Família | Quant | Tamanho | TTFT (ms) | TPOT (ms) | Acurácia | Score E3 | Status Tier 1 |")
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
        elif p.get("telemetry_count", 0) > 0 or (p.get("ema_latency_ms") or 0) > 0 or p.get("ttft_ms", 0) > 0:
            status = "❌ Reprovado (Guilhotina)"
        else:
            status = "⏳ Pendente"
            
        lines.append(f"| {idx} | `{m_name}` | {family} | {quant} | {size} | {ttft} | {tpot} | {acc} | **{e3}** | {status} |")

    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append(f"## 🧩 SEÇÃO 2: MÓDULOS AUXILIARES E SIDECARS ({len(sidecar_modules)})")
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
    lines.append(f"## 📦 SEÇÃO 3: MODELOS EMBARCADOS & CORE INTERNOS (src-tauri/models) ({len(embedded_models)})")
    lines.append("")
    lines.append("| # | Nome do Arquivo | Categoria | Formato | Tamanho | Descrição |")
    lines.append("| :--- | :--- | :---: | :---: | :---: | :--- |")
    
    if not embedded_models:
        lines.append("| - | Nenhum modelo interno encontrado em `src-tauri/models` | - | - | - | - |")
    else:
        for idx, em in enumerate(embedded_models, start=1):
            em_name = em["file_name"]
            em_cat = em["category"]
            em_fmt = em["extension"]
            em_size = format_bytes(em["file_size_bytes"])
            em_desc = em["description"]
            lines.append(f"| {idx} | `{em_name}` | `{em_cat}` | `{em_fmt}` | {em_size} | {em_desc} |")

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
        
        # Scores especializados se existirem
        scores_line = []
        if p.get("score_json_tools", 0.0) > 0:
            scores_line.append(f"JSON/Tools: `{p['score_json_tools'] * 100:.0f}%`")
        if p.get("score_code_ast", 0.0) > 0:
            scores_line.append(f"Code AST: `{p['score_code_ast'] * 100:.0f}%`")
        if p.get("score_reasoning", 0.0) > 0:
            scores_line.append(f"Reasoning CoT: `{p['score_reasoning'] * 100:.0f}%`")
        if p.get("score_vision_vqa", 0.0) > 0:
            scores_line.append(f"Vision VQA: `{p['score_vision_vqa'] * 100:.0f}%`")
        if scores_line:
            lines.append(f"- **Trilhas Cognitivas:** " + " | ".join(scores_line))

        att_str = "Nenhum"
        if p["attached_modules"]:
            att_str = ", ".join([f"`{mname}` ({msize})" for _, mname, msize in p["attached_modules"]])
        lines.append(f"- **Módulos Anexados:** {att_str}")
        
        t1 = p.get("tier1_passed")
        if t1 == 1:
            verdict = "RETENÇÃO RECOMENDADA: Excelente performance sintática e latência dentro da meta."
        elif p.get("telemetry_count", 0) > 0 or (p.get("ema_latency_ms") or 0) > 0 or p.get("ttft_ms", 0) > 0:
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

if __name__ == "__main__":
    generate_inventory_report()
