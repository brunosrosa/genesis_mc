#!/usr/bin/env python3
"""
SOULS LLM Inventory Viewer & Silicon Observability Dossier (V5 Multi-Engine & Multi-Tier)
==========================================================================================
Extrator e visualizador SSOT do banco `.souls_data/souls_heuristic_vault.db` e telemetria
empírica gerada pelo `souls_arena_cli`.

Gera relatório executivo em Markdown ('docs/observability/reports/souls_llms_inventory_summary.md')
com métricas completas de:
    - 5 Tiers Operacionais (Tier 0 ao Tier 5)
    - Matriz de Motores (ik_llama_vanguard, llama_upstream, mistral_rs, llama_cpp4)
    - 4 Trilhas Cognitivas (BFCL v4 Tools, Rust AST Code, CoT Reasoning E³, Vision VQA)
    - Aceleração Especulativa & MTP (Taxa Alpha & FinOps)
    - Disjuntores Térmicos & Circuit Breaker FFI
"""

import os
import sys
import json
import sqlite3
import argparse
from pathlib import Path
from typing import List, Dict, Any, Optional, Tuple
from datetime import datetime

def resolve_db_path(custom_path: Optional[str] = None) -> Path:
    """Localiza dinamicamente o banco de dados SSOT souls_heuristic_vault.db."""
    if custom_path:
        p = Path(custom_path)
        if p.exists():
            return p.resolve()
        print(f"[!] Caminho customizado não encontrado: {custom_path}", file=sys.stderr)

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
        return "0 MB"
    gb = bytes_val / (1024 ** 3)
    if gb >= 1.0:
        return f"{gb:.2f} GB"
    mb = bytes_val / (1024 ** 2)
    return f"{mb:.1f} MB"

def format_tokens(num: int) -> str:
    if num >= 1_000_000:
        return f"{num / 1_000_000:.2f}M"
    if num >= 1_000:
        return f"{num / 1_000:.1f}k"
    return str(num)

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

def infer_module_type_fallback(filepath: str) -> str:
    lower = str(filepath).lower()
    if "mmproj" in lower or "clip" in lower:
        return "VISION_PROJECTOR"
    elif "mtp" in lower:
        return "MTP_ADAPTER"
    elif "dspark" in lower or "draft" in lower or "dflash" in lower:
        return "SPECULATIVE_DRAFT"
    elif "bitnet" in lower or "i2_s" in lower or "i1_s" in lower:
        return "SPECIALIZED_QUANT"
    return "PRIMARY_LLM"

def classify_model_tier(model_info: Dict[str, Any]) -> str:
    """Classifica o modelo nos 5 Tiers Operacionais do SOULS."""
    filepath = str(model_info.get("file_path", "")).lower()
    filename = os.path.basename(filepath).lower()
    params = str(model_info.get("parameters", "")).lower()
    size_bytes = model_info.get("file_size_bytes", 0) or 0
    size_mb = size_bytes / (1024 * 1024)

    if "dspark" in filename or "dflash" in filename or "draft" in filename or model_info.get("module_type") == "SPECULATIVE_DRAFT":
        return "Tier 4 (Speculative Drafters)"

    if model_info.get("has_mmproj_sidecar") == 1 or "vision" in filename or "vl" in filename or "ui-tars" in filename:
        return "Tier 3 (Vision & Multimodal VLM)"

    # Critérios de tamanho de parâmetros / tensores
    if "135m" in filename or "360m" in filename or "k1" in filename or "gliclass" in filename or (0 < size_mb < 600):
        return "Tier 0 (Bootstrap & CPU Sanity)"
    elif "790m" in filename or "1b" in filename or "1.2b" in filename or "1.5b" in filename or (600 <= size_mb <= 1800):
        return "Tier 0.5 (Sensor Epistêmico)"
    elif "27b" in filename or "33b" in filename or "moe" in filename or "laguna" in filename or "14b" in filename or size_mb > 4500:
        return "Tier 2 (Background Agent & MoE Híbrido)"
    else:
        # Modelos 2B a 8B (Qwen 3.5 4B, Gemma 4 E2B chat, Phi-4-mini, Nemotron-4B, Fara-7B, Falcon3-Mamba-7B, Mamba-Codestral-7B, zamba2-2.7b)
        return "Tier 1 (Live Chat & Master)"

def fetch_inventory_data(conn: sqlite3.Connection) -> List[Dict[str, Any]]:
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    
    table_name = "model_registry" if check_table_exists(conn, "model_registry") else "local_models"
    has_telemetry = check_table_exists(conn, "arena_telemetry")
    
    # Inspeciona colunas disponíveis
    cursor.execute(f"PRAGMA table_info({table_name})")
    cols = {row["name"] for row in cursor.fetchall()}
    
    def get_col_sql(col_name: str, default_sql: str) -> str:
        return f"mr.{col_name}" if col_name in cols else f"{default_sql} as {col_name}"

    col_ttft = get_col_sql("ttft_ms", "0.0")
    col_tpot = get_col_sql("tpot_ms", "0.0")
    col_vram = get_col_sql("vram_peak_mb", "0.0")
    col_e3 = get_col_sql("e3_score", "0.0")
    col_engine = get_col_sql("engine_type", "'ik_llama_vanguard'")
    col_json = get_col_sql("score_json_tools", "0.0")
    col_code = get_col_sql("score_code_ast", "0.0")
    col_reason = get_col_sql("score_reasoning", "0.0")
    col_vision = get_col_sql("score_vision_vqa", "0.0")
    col_has_mmproj = get_col_sql("has_mmproj_sidecar", "0")
    col_mmproj_path = get_col_sql("mmproj_file_path", "NULL")
    col_module_type = get_col_sql("module_type", "'PRIMARY_LLM'")
    col_mtp_rate = get_col_sql("mtp_acceptance_rate", "0.0")
    col_cold_load = get_col_sql("vram_cold_load_ms", "0")
    col_deactivated_at = get_col_sql("deactivated_at", "0")
    col_deact_reason = get_col_sql("deactivation_reason", "NULL")

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
            {col_engine},
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
            {col_mtp_rate},
            {col_cold_load},
            {col_deactivated_at},
            {col_deact_reason},
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
            {col_engine},
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
            {col_mtp_rate},
            {col_cold_load},
            {col_deactivated_at},
            {col_deact_reason},
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

def fetch_telemetry_logs_summary(conn: sqlite3.Connection) -> Dict[str, Any]:
    """Coleta dados agregados da tabela telemetry_logs."""
    if not check_table_exists(conn, "telemetry_logs"):
        return {"total_calls": 0, "total_tokens_in": 0, "total_tokens_out": 0, "total_cost_usd": 0.0}
    
    cursor = conn.cursor()
    cursor.execute("""
        SELECT 
            COUNT(*) as total_calls,
            SUM(tokens_in) as total_tokens_in,
            SUM(tokens_out) as total_tokens_out,
            SUM(cost_usd) as total_cost_usd,
            AVG(duration_ms) as avg_duration_ms,
            AVG(accuracy_score) as avg_accuracy
        FROM telemetry_logs;
    """)
    row = cursor.fetchone()
    if row and row[0] > 0:
        return {
            "total_calls": row[0],
            "total_tokens_in": row[1] or 0,
            "total_tokens_out": row[2] or 0,
            "total_cost_usd": row[3] or 0.0,
            "avg_duration_ms": row[4] or 0.0,
            "avg_accuracy": row[5] or 1.0,
        }
    return {"total_calls": 0, "total_tokens_in": 0, "total_tokens_out": 0, "total_cost_usd": 0.0}

def scan_embedded_core_models(repo_root: Path) -> List[Dict[str, Any]]:
    """Varre modelos e tokenizers internos em src-tauri/models."""
    core_models_dir = repo_root / "src-tauri" / "models"
    items = []
    if core_models_dir.exists():
        for p in core_models_dir.rglob("*"):
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

def generate_inventory_report(custom_db: Optional[str] = None, output_file: Optional[str] = None) -> str:
    db_path = resolve_db_path(custom_db)
    repo_root = db_path.parent.parent
    print(f"[+] Conectando ao Banco SSOT em: {db_path}")
    
    conn = sqlite3.connect(db_path)
    all_rows = fetch_inventory_data(conn)
    logs_summary = fetch_telemetry_logs_summary(conn)
    conn.close()

    embedded_models = scan_embedded_core_models(repo_root)
    
    primary_models = []
    sidecar_modules = []
    
    for r in all_rows:
        mod_type = r.get("module_type") or infer_module_type_fallback(r.get("file_path", ""))
        r["inferred_type"] = mod_type
        r["assigned_tier"] = classify_model_tier(r)
        
        # Métrica de Acurácia Sintática (0.0% a 100.0%)
        if r.get("telemetry_count", 0) > 0 and r.get("syntax_success_rate") is not None:
            r["accuracy_pct"] = round(r["syntax_success_rate"] * 100.0, 2)
        else:
            ema_succ = r.get("success_rate_ema") or 0.0
            r["accuracy_pct"] = round(ema_succ * 100.0 if ema_succ <= 1.0 else ema_succ, 2)

        # TTFT e TPOT
        ttft = r.get("dyn_ttft_ms") if (r.get("dyn_ttft_ms") or 0.0) > 0 else r.get("ttft_ms", 0.0)
        tpot = r.get("dyn_tpot_ms") if (r.get("dyn_tpot_ms") or 0.0) > 0 else r.get("tpot_ms", 0.0)
        r["ttft_ms"] = round(ttft or 0.0, 2)
        r["tpot_ms"] = round(tpot or 0.0, 2)
        r["tps"] = round(1000.0 / r["tpot_ms"], 1) if r["tpot_ms"] > 0 else 0.0
        
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

    # Ordenação dos modelos por Tier e por E3 Score decrescente
    tier_order = {
        "Tier 0 (Bootstrap & CPU Sanity)": 0,
        "Tier 0.5 (Sensor Epistêmico)": 1,
        "Tier 1 (Live Chat & Master)": 2,
        "Tier 2 (Background Agent & MoE Híbrido)": 3,
        "Tier 3 (Vision & Multimodal VLM)": 4,
        "Tier 4 (Speculative Drafters)": 5,
    }

    primary_models.sort(
        key=lambda x: (
            tier_order.get(x.get("assigned_tier", ""), 99),
            -(x.get("tier1_passed") or 0),
            -(x.get("e3_score") or 0.0),
            -(x.get("tps") or 0.0),
        )
    )
    
    # Mapeamento de Sidecars
    for p in primary_models:
        p_dir = os.path.dirname(p.get("file_path", ""))
        attached = []
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
    summary_md_file = Path(output_file) if output_file else reports_dir / "souls_llms_inventory_summary.md"
    
    lines = []
    lines.append("# 📊 SOULS SILICON OBSERVABILITY & LLM INVENTORY DOSSIER (V5)")
    lines.append(f"**Data de Geração:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')} | **Banco SSOT:** `{db_path}`")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 🖥️ 1. RESUMO EXECUTIVO DE SILÍCIO & GOVERNANÇA BARE-METAL")
    lines.append("- **Aceleração Gráfica (Target GPU):** NVIDIA GeForce RTX 2060m (6GB VRAM GDDR6, Arquitetura Turing sm_75)")
    lines.append("- **Aceleração Host (CPU):** Intel Core i9 (AVX2 SIMD AOT) + Gateway Tokio Rust")
    lines.append("- **Matriz de Motores:**")
    lines.append("  - `ik_llama_vanguard`: TurboQuant com V-Cache 4-bit, FlashAttention O(1) e LoRA residual.")
    lines.append("  - `llama_upstream`: Binding oficial llama.cpp 2026 para arquiteturas Phi-4, Nemotron, LFM e Mamba GGUF.")
    lines.append("  - `mistral_rs`: Runtime bare-metal especializado em State Space Models (SSM/Mamba).")
    lines.append("  - `llama_cpp4`: Motor puro de CPU AVX2 para calibração, logit probing e fallback de sensor.")
    lines.append("- **Eficiência FinOps ($E^3$ Score):** $E^3 = \\frac{\\text{Acurácia}^2}{\\text{Latência Média (s)} + 0.001}$")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 📈 2. MÉTRICAS GLOBAIS DO INVENTÁRIO DE MODELOS")
    lines.append("")
    lines.append("| Total GGUF | LLMs Principais | Aprovados Tier 1 | Reprovados/Quarentena | Pendentes | Sidecars | Modelos Core (src-tauri) |")
    lines.append("| :---: | :---: | :---: | :---: | :---: | :---: | :---: |")
    
    app_count = sum(1 for p in primary_models if p.get("tier1_passed") == 1)
    rep_count = sum(1 for p in primary_models if p.get("tier1_passed") == 0 and ((p.get("ema_latency_ms") or 0) > 0 or p.get("telemetry_count", 0) > 0 or p.get("ttft_ms", 0) > 0 or (p.get("deactivated_at") or 0) > 0))
    pen_count = sum(1 for p in primary_models if p.get("tier1_passed") == 0 and (p.get("ema_latency_ms") or 0) == 0 and p.get("telemetry_count", 0) == 0 and p.get("ttft_ms", 0) == 0 and (p.get("deactivated_at") or 0) == 0)
    
    lines.append(f"| **{len(all_rows)}** | **{len(primary_models)}** | **{app_count}** | **{rep_count}** | **{pen_count}** | **{len(sidecar_modules)}** | **{len(embedded_models)}** |")
    lines.append("")

    if logs_summary["total_calls"] > 0:
        lines.append("### ⚡ Telemetria Agregada em Produção (`telemetry_logs`)")
        lines.append(f"- **Execuções Registradas:** `{logs_summary['total_calls']}` chamadas")
        lines.append(f"- **Tokens de Entrada:** `{format_tokens(logs_summary['total_tokens_in'])}` | **Tokens de Saída:** `{format_tokens(logs_summary['total_tokens_out'])}`")
        lines.append(f"- **Custo FinOps Acumulado:** `${logs_summary['total_cost_usd']:.6f} USD` | **Latência Média:** `{logs_summary['avg_duration_ms']:.1f} ms`")
        lines.append("")

    lines.append("---")
    lines.append("")
    lines.append("## 🏆 3. LEADERBOARD POR TIER & MATRIZ DE MOTORES")
    lines.append("")

    # Agrupa por Tier
    current_tier = None
    for p in primary_models:
        tier = p.get("assigned_tier", "Tier Desconhecido")
        if tier != current_tier:
            current_tier = tier
            lines.append(f"### 🎯 {current_tier}")
            lines.append("")
            lines.append("| # | Modelo | Família | Quant | Tamanho | Motor Campeão | TTFT (ms) | TPOT (ms) | TPS | VRAM Pico | Score E³ | Status |")
            lines.append("| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :--- |")
            tier_idx = 1
        
        m_name = p.get("model_name") or os.path.basename(str(p.get("file_path", "")))
        family = p.get("family") or "N/A"
        quant = p.get("quantization") or "N/A"
        size = format_bytes(p.get("file_size_bytes"))
        engine = p.get("engine_type") or "ik_llama_vanguard"
        ttft = f"{p['ttft_ms']:.1f}" if p['ttft_ms'] > 0 else "N/A"
        tpot = f"{p['tpot_ms']:.1f}" if p['tpot_ms'] > 0 else "N/A"
        tps = f"{p['tps']:.1f}" if p['tps'] > 0 else "N/A"
        vram = f"{p.get('vram_peak_mb', 0.0):.0f} MB" if (p.get("vram_peak_mb") or 0.0) > 0 else "0 MB"
        e3 = f"{p['e3_score']:.4f}"
        
        if (p.get("deactivated_at") or 0) > 0:
            status = "🔴 Quarentena (Disjuntor)"
        elif p.get("tier1_passed") == 1:
            status = "✅ Aprovado (Campeão)"
        elif p.get("telemetry_count", 0) > 0 or (p.get("ema_latency_ms") or 0) > 0 or p.get("ttft_ms", 0) > 0:
            status = "❌ Reprovado"
        else:
            status = "⏳ Pendente"
            
        lines.append(f"| {tier_idx} | `{m_name}` | {family} | {quant} | {size} | `{engine}` | {ttft} | {tpot} | {tps} | {vram} | **{e3}** | {status} |")
        tier_idx += 1

    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 🧬 4. COLISEU COGNITIVO: MATRIZ DAS 4 TRILHAS QUALITATIVAS (TIER 2)")
    lines.append("")
    lines.append("| Modelo | Tier | 🛠️ Tools (BFCL v4) | 🦀 Rust AST Code | 🧠 CoT Reasoning E³ | 👁️ VLM VQA | Veredito Cognitivo |")
    lines.append("| :--- | :---: | :---: | :---: | :---: | :---: | :--- |")
    
    for p in primary_models:
        m_name = p.get("model_name") or os.path.basename(str(p.get("file_path", "")))
        tier_short = p.get("assigned_tier", "").split("(")[0].strip()
        s_json = f"{p['score_json_tools'] * 100:.0f}%" if (p.get("score_json_tools") or 0.0) > 0 else "N/A"
        s_code = f"{p['score_code_ast'] * 100:.0f}%" if (p.get("score_code_ast") or 0.0) > 0 else "N/A"
        s_reason = f"{p['score_reasoning'] * 100:.0f}%" if (p.get("score_reasoning") or 0.0) > 0 else "N/A"
        s_vision = f"{p['score_vision_vqa'] * 100:.0f}%" if (p.get("score_vision_vqa") or 0.0) > 0 else "N/A"
        
        has_eval = any(v != "N/A" for v in [s_json, s_code, s_reason, s_vision])
        verdict = "Pronto para Roteamento" if has_eval else "Aguardando Avaliação Qualitativa"
        lines.append(f"| `{m_name}` | {tier_short} | {s_json} | {s_code} | {s_reason} | {s_vision} | {verdict} |")

    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## ⚡ 5. ACELERAÇÃO ESPECULATIVA & MTP DRAFTING (TIER 4)")
    lines.append("")
    lines.append("| Modelo Rascunho | Formato | Taxa de Aceitação Alpha (α) | Speedup Projetado | Veredito FinOps |")
    lines.append("| :--- | :---: | :---: | :---: | :--- |")
    
    drafter_models = [p for p in primary_models if "Tier 4" in p.get("assigned_tier", "")] + [s for s in sidecar_modules if s.get("inferred_type") == "SPECULATIVE_DRAFT"]
    if not drafter_models:
        lines.append("| - | Nenhum modelo de drafting registrado | - | - | - |")
    else:
        for d in drafter_models:
            d_name = d.get("model_name") or os.path.basename(str(d.get("file_path", "")))
            alpha = d.get("mtp_acceptance_rate") or 0.0
            alpha_str = f"{alpha * 100:.1f}%" if alpha > 0 else "N/A (Aguardando Tier 4)"
            speedup = f"{1.0 / (1.0 - alpha + 0.1):.2f}x" if alpha > 0 else "1.00x"
            verdict = "✅ Aprovado para Produção (α ≥ 55%)" if alpha >= 0.55 else ("❌ Descarte FinOps (α < 55%)" if alpha > 0 else "⏳ Pendente de Combate")
            lines.append(f"| `{d_name}` | GGUF Draft | {alpha_str} | {speedup} | {verdict} |")

    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append(f"## 🧩 6. PAREAMENTO MULTIMODAL & MÓDULOS AUXILIARES ({len(sidecar_modules)})")
    lines.append("")
    lines.append("| # | Nome do Módulo | Tipo | Tamanho | Caminho Físico |")
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
    lines.append(f"## 📦 7. MODELOS EMBARCADOS & CORE INTERNOS (`src-tauri/models`) ({len(embedded_models)})")
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
    lines.append("## 🛡️ 8. DISJUNTORES DE SAÚDE & QUARENTENAS TÉRMICAS (CIRCUIT BREAKERS)")
    lines.append("")
    quarantined = [p for p in primary_models if (p.get("deactivated_at") or 0) > 0 or p.get("deactivation_reason")]
    if not quarantined:
        lines.append("✅ **Nenhum modelo em quarentena.** Todos os modelos ativos operam dentro da barreira térmica e de estabilidade.")
    else:
        lines.append("| Modelo | Motivo da Desativação | Data de Quarentena | Ação Recomendada |")
        lines.append("| :--- | :--- | :---: | :--- |")
        for q in quarantined:
            q_name = q.get("model_name") or os.path.basename(str(q.get("file_path", "")))
            reason = q.get("deactivation_reason") or "Timeout / Falha FFI"
            dt_str = datetime.fromtimestamp(q.get("deactivated_at", 0)).strftime('%Y-%m-%d %H:%M:%S') if q.get("deactivated_at") else "N/A"
            lines.append(f"| `{q_name}` | `{reason}` | {dt_str} | Revisar suporte do motor ou purgar do SSD |")

    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 📝 9. FICHA TÉCNICA E DOSSIÊ INDIVIDUAL DOS MODELOS")
    lines.append("")
    
    for idx, p in enumerate(primary_models, start=1):
        m_name = p.get("model_name") or os.path.basename(str(p.get("file_path", "")))
        lines.append(f"### {idx}. `{m_name}`")
        lines.append(f"- **Tier Operacional:** `{p.get('assigned_tier')}` | **Motor Campeão:** `{p.get('engine_type', 'ik_llama_vanguard')}`")
        lines.append(f"- **Caminho Físico:** `{p.get('file_path')}`")
        lines.append(f"- **Metadados:** Família `{p.get('family')}` | Parâmetros `{p.get('parameters')}` | Contexto Máximo `{p.get('context_length')}` tokens | Quant `{p.get('quantization')}`")
        lines.append(f"- **Desempenho de Silício:** TTFT `{p['ttft_ms']} ms` | TPOT `{p['tpot_ms']} ms` | Throughput `{p['tps']} tok/s` | VRAM Pico `{p.get('vram_peak_mb', 0.0):.0f} MB` | **Score E³ `{p['e3_score']}`**")
        
        scores_line = []
        if (p.get("score_json_tools") or 0.0) > 0:
            scores_line.append(f"Tools BFCL: `{p['score_json_tools'] * 100:.0f}%`")
        if (p.get("score_code_ast") or 0.0) > 0:
            scores_line.append(f"Rust AST: `{p['score_code_ast'] * 100:.0f}%`")
        if (p.get("score_reasoning") or 0.0) > 0:
            scores_line.append(f"Reasoning CoT: `{p['score_reasoning'] * 100:.0f}%`")
        if (p.get("score_vision_vqa") or 0.0) > 0:
            scores_line.append(f"Vision VQA: `{p['score_vision_vqa'] * 100:.0f}%`")
        if scores_line:
            lines.append(f"- **Avaliação Qualitativa:** " + " | ".join(scores_line))

        att_str = "Nenhum"
        if p["attached_modules"]:
            att_str = ", ".join([f"`{mname}` ({msize})" for _, mname, msize in p["attached_modules"]])
        lines.append(f"- **Módulos Anexados:** {att_str}")
        
        if (p.get("deactivated_at") or 0) > 0:
            verdict = f"🔴 QUARENTENA: Desativado por `{p.get('deactivation_reason', 'falha FFI')}`."
        elif p.get("tier1_passed") == 1:
            verdict = f"🟢 RETENÇÃO RECOMENDADA: Modelo aprovado com E³ `{p['e3_score']}` despachado pelo `{p.get('engine_type', 'ik_llama_vanguard')}`."
        elif p.get("telemetry_count", 0) > 0 or (p.get("ema_latency_ms") or 0) > 0 or p.get("ttft_ms", 0) > 0:
            verdict = "🔴 REPROVADO NA ARENA: Não atingiu os critérios mínimos de precisão sintática ou throughput."
        else:
            verdict = "🟡 AGUARDANDO ARENA: Modelo aguarda execução de benchmark."
        lines.append(f"- **Veredito ParetoBandit:** {verdict}")
        lines.append("")

    lines.append("---")
    lines.append("*Fim do Dossiê de Inventário SOULS V5. Gerado automaticamente via `souls_llms_inventory_viewer.py`.*")
    
    report_content = "\n".join(lines)
    with open(summary_md_file, "w", encoding="utf-8") as f:
        f.write(report_content)
        
    print(f"[+] Relatório Markdown de Inventário e Telemetria gerado com sucesso em:")
    print(f"    {summary_md_file}")
    return str(summary_md_file)

def main():
    parser = argparse.ArgumentParser(description="SOULS LLM Inventory & Silicon Observability Dossier")
    parser.add_argument("--db", type=str, default=None, help="Caminho customizado para o souls_heuristic_vault.db")
    parser.add_argument("--output", "-o", type=str, default=None, help="Caminho do arquivo Markdown de saída")
    parser.add_argument("--json", action="store_true", help="Exporta o inventário completo em formato JSON para stdout")
    args = parser.parse_args()

    if args.json:
        db_path = resolve_db_path(args.db)
        conn = sqlite3.connect(db_path)
        data = fetch_inventory_data(conn)
        conn.close()
        for r in data:
            r["assigned_tier"] = classify_model_tier(r)
        print(json.dumps(data, indent=2, ensure_ascii=False))
    else:
        generate_inventory_report(args.db, args.output)

if __name__ == "__main__":
    main()
