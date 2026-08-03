"""Auditoria qualitativa 0-100 dos 11 blobs do Harvester (Fase 0).

Lê todos os pares (repo_id, artifact_type) do SQLite, computa 8 dimensões de
qualidade por par, agrega por artifact_type para detectar padrões sistêmicos,
e gera 3 visões:

1. Tabela horizontal: 11 colunas (1 por blob) × N linhas (1 por repo)
2. Agregado por artifact_type: média/std/min/max (responde "qual blob é o gargalo")
3. Ranking de piores casos: top-20 (repo, blob) com score < 50

Gera:
- docs/observability/audits/quality/_QUALITY_SCORES.md (relatório human-readable)
- docs/observability/audits/quality/_QUALITY_SCORES.json (dados machine-readable)
- stdout: sumário executivo

Implementação: zero dependência externa (só stdlib: sqlite3, re, json, pathlib,
collections, datetime, statistics). Per ADR-001-Core-Stack-Restrita e Anti-Slop Protocol.

Invocação:
    python docs/runtime/scripts/audit_blob_quality.py [--db-path PATH] [--out-dir PATH]
                                              [--top N] [--min-score N]
                                              [--repo-allowlist REPO [REPO ...]]
"""
from __future__ import annotations

import argparse
import json
import re
import sqlite3
import statistics
import sys
from collections import Counter, defaultdict
from datetime import datetime
from pathlib import Path
from typing import Any, Iterable

# -----------------------------------------------------------------------------
# Constantes canônicas (cross-ref ADR-031 §4)
# -----------------------------------------------------------------------------

# Ordem canônica dos 11 blobs (alinhada com ADR-031 §4)
BLOB_TYPES: list[str] = [
    "blob_01_promessa_readme",
    "blob_02_dependency_manifest",
    "blob_03_test_intent",
    "blob_04_repo_outline",
    "blob_05_architecture_map",
    "blob_06_unsafe_hotspots",
    "blob_07_ops_blueprint",
    "blob_08_health_report",
    "blob_09_community_meta",
    "blob_10_souls_canon_context",
    "blob_11_ux_contracts",
]

# Faixas de tamanho sadio (bytes) por blob.
# Heurística baseada em observações do ETL atual (trailbase 2026-07-16).
# PLACEHOLDER_MIN: abaixo disso = claramente vazio
# EXPECTED_MIN: abaixo disso = sub-ótimo (suspeito)
# EXPECTED_MAX: acima disso = bloated
SIZE_HEURISTICS: dict[str, dict[str, int]] = {
    "blob_01_promessa_readme":      {"placeholder": 200,  "min": 1_000,     "max": 100_000},
    "blob_02_dependency_manifest":  {"placeholder": 200,  "min": 1_000,     "max": 200_000},
    "blob_03_test_intent":          {"placeholder": 200,  "min": 5_000,     "max": 500_000},
    "blob_04_repo_outline":         {"placeholder": 200,  "min": 10_000,    "max": 1_000_000},
    "blob_05_architecture_map":     {"placeholder": 200,  "min": 1_000,     "max": 200_000},
    "blob_06_unsafe_hotspots":      {"placeholder": 50,   "min": 100,       "max": 50_000},
    "blob_07_ops_blueprint":        {"placeholder": 200,  "min": 1_000,     "max": 100_000},
    "blob_08_health_report":        {"placeholder": 200,  "min": 1_000,     "max": 2_000_000},
    "blob_09_community_meta":       {"placeholder": 200,  "min": 1_000,     "max": 50_000},
    "blob_10_souls_canon_context":   {"placeholder": 1_000,"min": 4_000,     "max": 50_000},  # canon é uniforme cross-repo
    "blob_11_ux_contracts":         {"placeholder": 0,    "min": 1_000,     "max": 200_000},  # 0 se repo sem UI
}

# Marcadores estruturais canônicos por blob (regex). Cada marcador presente = pontos.
STRUCTURE_MARKERS: dict[str, list[str]] = {
    "blob_01_promessa_readme":     [r"^#\s+\S", r"```", r"\n##\s+"],
    "blob_02_dependency_manifest": [r"\[(?:dependencies|dev-dependencies|package)\]", r"=\s*[\"\d]", r"^\s*\[", r"version\s*="],
    "blob_03_test_intent":         [r"#\[(?:test|case)\]", r"fn\s+\w+", r"async\s+fn", r"describe\(|it\("],
    "blob_04_repo_outline":        [r"^\s*(?:pub\s+)?(?:fn|struct|enum|trait|impl|class|def|interface)\s+\w+", r"^\s*pub\s+(?:use|mod|fn|struct|enum)"],
    "blob_05_architecture_map":     [r"->|=>|import|require|from|use\s+"],
    "blob_06_unsafe_hotspots":     [r"\[DOMAIN:\s*[^\]]+\]", r"::\s*L?\d+", r"(?i)(?:unsafe|eval|exec|hardcoded|key|injection)"],
    "blob_07_ops_blueprint":       [r"(?i)(?:FROM|RUN|COPY|WORKDIR|ENTRYPOINT|CMD|name:|on:|jobs:|steps:)", r"Dockerfile|Makefile|workflow"],
    "blob_08_health_report":       [r"\[DOMAIN:\s*[^\]]+\]", r"::\s*L?\d+", r"(?i)(?:WARNING|ERROR|INFO|complexity|cyclomatic|dead)"],
    "blob_09_community_meta":      [r'"(?:stargazers_count|forks_count|open_issues_count|pushed_at|updated_at|html_url|stars|forks|watchers)"\s*:\s*'],
    "blob_10_souls_canon_context":  [r"^#\s+\S", r"^##\s+", r"SOULS|Souls"],
    "blob_11_ux_contracts":        [r"(?i)(?:Props|Events|Dispatch|emits|defineProps|defineEmits|interface\s+\w+Props|component\s+|export\s+(?:function|const)|use\w+|\$props)"],
}

# Marcadores de FAIL (Lei IV: zero-byte uniforme)
FAIL_PATTERNS: list[str] = [
    r"(?i)Warning:\s*Timeout",
    r"(?i)Erro:\s*0 matches",
    r"(?i)ERROR:\s*Timeout",
    r"(?i)Failed:\s*timeout",
]

# Marcadores de slop (penalizam)
SLOP_PATTERNS: list[str] = [
    r"\bTODO\b", r"\bFIXME\b", r"\bPLACEHOLDER\b", r"\bXXX\b",
    r"<empty>", r"\bHACK\b", r"\bWIP\b",
]

# Marcadores de rebrand (binário: 0 ou 100)
REBRAND_FORBIDDEN: list[str] = [
    r"\bgenesis_mc\b",
    r"\bgenesis-mc\b",
    r"Genesis\s+MC\b",
    r"GenesisMC\b",
]

# Pesos por dimensão (soma = 100)
WEIGHTS: dict[str, float] = {
    "tamanho_sadio":        20.0,
    "estrutura_canonica":   20.0,
    "lei_iv_compliance":    20.0,  # hard-fail: se 0, score final cap em 50
    "diversidade_fonte":    10.0,
    "refs_file_line":       10.0,
    "sem_slop":             10.0,
    "rebrand_clean":        5.0,
    "retrocompat_schema":   5.0,
}


# -----------------------------------------------------------------------------
# Sampling (eficiência de memória: não carrega payload inteiro se for > 6KB)
# -----------------------------------------------------------------------------

SAMPLE_HEAD = 2_000
SAMPLE_TAIL = 2_000
SAMPLE_MID = 2_000
SAMPLE_THRESHOLD = 6_000


def sample_payload(payload: str | bytes) -> str:
    """Retorna head + middle + tail do payload para análise eficiente."""
    if isinstance(payload, (bytes, bytearray)):
        try:
            payload = payload.decode("utf-8")
        except UnicodeDecodeError:
            payload = payload.decode("utf-8", errors="replace")
    if len(payload) <= SAMPLE_THRESHOLD:
        return payload
    head = payload[:SAMPLE_HEAD]
    mid_start = (len(payload) - SAMPLE_MID) // 2
    mid = payload[mid_start:mid_start + SAMPLE_MID]
    tail = payload[-SAMPLE_TAIL:]
    return f"{head}\n... [MIDDLE TRUNCATED] ...\n{mid}\n... [CONTINUES] ...\n{tail}"


# -----------------------------------------------------------------------------
# Scoring functions (1 por dimensão)
# -----------------------------------------------------------------------------

def score_tamanho_sadio(size: int, blob: str) -> tuple[float, str]:
    h = SIZE_HEURISTICS.get(blob)
    if not h:
        return 50.0, "no_heuristic"
    if size == 0:
        return 0.0, "empty"
    if size < h["placeholder"]:
        return 10.0, f"placeholder({size}<{h['placeholder']})"
    if size < h["min"]:
        return 50.0, f"below_min({size}<{h['min']})"
    if size > h["max"] * 5:
        return 20.0, f"bloated({size}>{h['max']*5})"
    if size > h["max"]:
        return 60.0, f"above_max({size}>{h['max']})"
    return 100.0, f"healthy({size})"


def score_estrutura_canonica(text: str, blob: str) -> tuple[float, str]:
    markers = STRUCTURE_MARKERS.get(blob, [])
    if not markers:
        return 50.0, "no_markers"
    hits = sum(1 for m in markers if re.search(m, text, re.MULTILINE))
    score = (hits / len(markers)) * 100.0
    return score, f"{hits}/{len(markers)} markers"


def score_lei_iv_compliance(text: str) -> tuple[float, str]:
    """Hard-fail dimension. Lei IV do ADR-031: zero 'Warning: Timeout' no payload."""
    clean_lines = []
    for line in text.splitlines():
        trimmed = line.strip()
        if trimmed.startswith("[") and ("DIAGNÓSTICO" in trimmed or "FALHA_NORMALIZACAO" in trimmed):
            continue
        clean_lines.append(line)
    clean_text = "\n".join(clean_lines)

    for p in FAIL_PATTERNS:
        if re.search(p, clean_text):
            return 0.0, f"violation: {p}"
    return 100.0, "clean"



def score_diversidade_fonte(text: str) -> tuple[float, str]:
    """Heurística: conta fontes distintas (opengrep, govulncheck, clippy, biome, oxc, etc)."""
    sources = set()
    for s in ("opengrep", "govulncheck", "clippy", "biome", "oxc", "rustc", "eslint", "tsc",
              "mypy", "ruff", "gopls", "golangci", "reqwest", "github"):
        if re.search(rf"\b{re.escape(s)}\b", text, re.IGNORECASE):
            sources.add(s)
    n = len(sources)
    if n == 0:
        # pode ser um blob que não tem múltiplas fontes (ex: README)
        return 50.0, "no_tool_markers"
    if n == 1:
        return 30.0, f"monocultura({next(iter(sources))})"
    if n == 2:
        return 60.0, f"duo({','.join(sorted(sources))})"
    if n >= 3:
        return 100.0, f"diverse({n})"
    return 70.0, f"moderate({n})"


def score_refs_file_line(text: str) -> tuple[float, str]:
    """Heurística: conta referências a arquivo:linha (:: L\\d+ ou file.ext:line)."""
    refs = re.findall(r"(?:\w[\w./_-]+\.\w+)\s*::\s*L?\d+|\b\w+\.\w+:\d+", text)
    n = len(refs)
    if n == 0:
        return 30.0, "no_refs"
    if n < 5:
        return 50.0, f"few_refs({n})"
    if n < 50:
        return 75.0, f"some_refs({n})"
    return 100.0, f"rich_refs({n})"


def score_sem_slop(text: str) -> tuple[float, str]:
    counts = {p: len(re.findall(p, text)) for p in SLOP_PATTERNS}
    total = sum(counts.values())
    if total == 0:
        return 100.0, "clean"
    if total <= 3:
        return 80.0, f"minor({total})"
    if total <= 10:
        return 50.0, f"moderate({total})"
    return 0.0, f"slop({total})"


def score_rebrand_clean(text: str) -> tuple[float, str]:
    for p in REBRAND_FORBIDDEN:
        if re.search(p, text):
            return 0.0, f"violation: {p}"
    return 100.0, "clean"


def score_retrocompat_schema(text: str, blob: str) -> tuple[float, str]:
    """Heurística: parseabilidade compatível com detect_payload_column."""
    if blob == "blob_09_community_meta":
        # esperado JSON
        try:
            json.loads(text)
            return 100.0, "json_parseable"
        except json.JSONDecodeError:
            return 0.0, "not_json"
    if blob in ("blob_06_unsafe_hotspots", "blob_08_health_report"):
        if "[DOMAIN:" in text or re.search(r"^\s*-\s*\[(?:info|warning|error)", text, re.MULTILINE | re.IGNORECASE):
            return 100.0, "report_format"
        return 30.0, "unknown_format"
    # Outros: texto livre
    return 100.0, "text"


def score_blob(repo_id: str, blob: str, size: int, sample: str) -> dict[str, Any]:
    """Computa o score agregado de um par (repo, blob)."""
    # Detecta se é um relatório de saúde perfeitamente limpo (sem findings ou vazio de forma sadia)
    is_healthy_report = False
    if blob == "blob_08_health_report" and ("summary: findings=0" in sample or "Sem divida tecnica" in sample):
        is_healthy_report = True

    is_json_community = False
    if blob == "blob_09_community_meta":
        try:
            json.loads(sample)
            is_json_community = True
        except Exception:
            pass

    is_valid_ux_inventory = False
    if blob == "blob_11_ux_contracts" and ("component " in sample or "props " in sample or "interface " in sample):
        is_valid_ux_inventory = True

    dims = {
        "tamanho_sadio":      (100.0, "healthy(findings=0)") if is_healthy_report else ((100.0, "valid_ux") if is_valid_ux_inventory else score_tamanho_sadio(size, blob)),
        "estrutura_canonica": (100.0, "clean(findings=0)") if is_healthy_report else score_estrutura_canonica(sample, blob),
        "lei_iv_compliance":  score_lei_iv_compliance(sample),
        "diversidade_fonte":  (100.0, "clean(findings=0)") if is_healthy_report else ((100.0, "github_api") if is_json_community else score_diversidade_fonte(sample)),
        "refs_file_line":     (100.0, "clean(findings=0)") if is_healthy_report else ((100.0, "github_api") if is_json_community else ((100.0, "ux_inventory") if is_valid_ux_inventory else score_refs_file_line(sample))),
        "sem_slop":           score_sem_slop(sample),
        "rebrand_clean":      score_rebrand_clean(sample),
        "retrocompat_schema": score_retrocompat_schema(sample, blob),
    }
    raw_total = sum(score * WEIGHTS[k] / 100.0 for k, (score, _) in dims.items())
    # Hard-cap em 50 se Lei IV violada
    if dims["lei_iv_compliance"][0] == 0.0:
        raw_total = min(raw_total, 50.0)
    return {
        "repo_id": repo_id,
        "blob": blob,
        "size": size,
        "dimensions": {k: {"score": v[0], "detail": v[1]} for k, v in dims.items()},
        "total_score": round(raw_total, 2),
    }


# -----------------------------------------------------------------------------
# I/O
# -----------------------------------------------------------------------------

def connect(db_path: Path) -> sqlite3.Connection:
    if not db_path.exists():
        raise FileNotFoundError(f"SQLite not found: {db_path}")
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    return conn


def fetch_latest(conn: sqlite3.Connection, repo_allowlist: list[str] | None) -> list[dict[str, Any]]:
    """Retorna o payload mais recente de cada (repo_id, artifact_type)."""
    where = ""
    params: tuple = ()
    if repo_allowlist:
        placeholders = ",".join("?" for _ in repo_allowlist)
        where = f"WHERE repo_id IN ({placeholders})"
        params = tuple(repo_allowlist)
    q = f"""
        SELECT repo_id, artifact_type, payload_blob, length(payload_blob) AS sz, timestamp_extracao, artifact_id
        FROM artefatos_brutos
        {where}
        ORDER BY repo_id, artifact_type, timestamp_extracao DESC
    """
    rows = conn.execute(q, params).fetchall()
    seen: set[tuple[str, str]] = set()
    latest: list[dict[str, Any]] = []
    for r in rows:
        key = (r["repo_id"], r["artifact_type"])
        if key in seen:
            continue
        seen.add(key)
        latest.append(dict(r))
    return latest


def render_markdown(scores: list[dict[str, Any]], top: int) -> str:
    """Tabela horizontal + agregado + top piores."""
    out: list[str] = []
    out.append("# Auditoria Qualitativa dos Blobs (Fase 0 Harvester)\n")
    out.append(f"**Gerado em:** {datetime.now().isoformat(timespec='seconds')}\n")
    out.append(f"**Pares (repo, blob) auditados:** {len(scores)}\n")
    out.append("**Referência:** spec-040 / ADR-031 §4 (anatomia dos 11 blobs)\n")
    out.append("")

    # 1) Tabela horizontal: 11 colunas (1 por blob) × N linhas (1 por repo)
    out.append("## 1. Score por (repo, blob)\n")
    by_repo: dict[str, dict[str, float]] = defaultdict(dict)
    for s in scores:
        by_repo[s["repo_id"]][s["blob"]] = s["total_score"]
    out.append("| repo_id | " + " | ".join(b.replace("blob_", "") for b in BLOB_TYPES) + " | avg |")
    out.append("|---" * (len(BLOB_TYPES) + 2) + "|")
    for repo in sorted(by_repo.keys()):
        cells = []
        for b in BLOB_TYPES:
            sc = by_repo[repo].get(b, -1.0)
            if sc < 0:
                cells.append("—")
            else:
                cells.append(f"{sc:.0f}")
        avg = statistics.mean([v for v in by_repo[repo].values() if v >= 0]) if by_repo[repo] else 0
        out.append(f"| `{repo}` | " + " | ".join(cells) + f" | **{avg:.1f}** |")
    out.append("")

    # 2) Agregado por artifact_type
    out.append("## 2. Agregado por artifact_type (sistêmico)\n")
    out.append("| blob | média | std | min | max | n |")
    out.append("|---|---:|---:|---:|---:|---:|")
    by_blob: dict[str, list[float]] = defaultdict(list)
    for s in scores:
        by_blob[s["blob"]].append(s["total_score"])
    for b in BLOB_TYPES:
        vals = by_blob.get(b, [])
        if not vals:
            out.append(f"| {b} | — | — | — | — | 0 |")
            continue
        out.append(
            f"| {b} | **{statistics.mean(vals):.1f}** | {statistics.pstdev(vals):.1f} "
            f"| {min(vals):.0f} | {max(vals):.0f} | {len(vals)} |"
        )
    out.append("")

    # 3) Top piores casos
    out.append(f"## 3. Top {top} piores casos (score < 50)\n")
    out.append("| repo_id | blob | size | score | detail |")
    out.append("|---|---|---:|---:|---|")
    worst = sorted([s for s in scores if s["total_score"] < 50], key=lambda x: x["total_score"])[:top]
    if not worst:
        out.append("| (nenhum abaixo de 50) | | | | |")
    for s in worst:
        lei_iv = s["dimensions"]["lei_iv_compliance"]["detail"]
        slop = s["dimensions"]["sem_slop"]["detail"]
        tam = s["dimensions"]["tamanho_sadio"]["detail"]
        detail = f"tam={tam}; lei_iv={lei_iv}; slop={slop}"
        out.append(f"| `{s['repo_id']}` | {s['blob']} | {s['size']} | **{s['total_score']}** | {detail} |")
    out.append("")

    # 4) Top melhores casos
    out.append(f"## 4. Top {top} melhores casos (score ≥ 80)\n")
    out.append("| repo_id | blob | size | score |")
    out.append("|---|---|---:|---:|")
    best = sorted([s for s in scores if s["total_score"] >= 80], key=lambda x: -x["total_score"])[:top]
    if not best:
        out.append("| (nenhum ≥ 80) | | | |")
    for s in best:
        out.append(f"| `{s['repo_id']}` | {s['blob']} | {s['size']} | **{s['total_score']}** |")
    out.append("")

    # 5) Resumo executivo
    out.append("## 5. Resumo executivo\n")
    if by_blob:
        ranked = sorted(((b, statistics.mean(v)) for b, v in by_blob.items() if v), key=lambda x: x[1])
        worst_blob, worst_avg = ranked[0] if ranked else ("?", 0)
        best_blob, best_avg = ranked[-1] if ranked else ("?", 0)
        out.append(f"- **Blob mais fraco (sistêmico):** `{worst_blob}` com média {worst_avg:.1f}")
        out.append(f"- **Blob mais forte (sistêmico):** `{best_blob}` com média {best_avg:.1f}")
    lei_iv_violations = sum(1 for s in scores if s["dimensions"]["lei_iv_compliance"]["score"] == 0)
    rebrand_violations = sum(1 for s in scores if s["dimensions"]["rebrand_clean"]["score"] == 0)
    slop_cases = sum(1 for s in scores if s["dimensions"]["sem_slop"]["score"] < 50)
    out.append(f"- **Violações de Lei IV (ADR-031):** {lei_iv_violations} / {len(scores)}")
    out.append(f"- **Violações de rebrand (`genesis_mc` residual):** {rebrand_violations} / {len(scores)}")
    out.append(f"- **Casos com slop (TODO/FIXME/PLACEHOLDER):** {slop_cases} / {len(scores)}")
    out.append("")
    return "\n".join(out)


def print_summary(scores: list[dict[str, Any]], top: int) -> None:
    by_blob: dict[str, list[float]] = defaultdict(list)
    for s in scores:
        by_blob[s["blob"]].append(s["total_score"])
    print(f"\n=== AUDIT SUMMARY ===")
    print(f"Total pares (repo, blob): {len(scores)}")
    if by_blob:
        ranked = sorted(((b, statistics.mean(v)) for b, v in by_blob.items() if v), key=lambda x: x[1])
        print("\nRanking de blobs por média (pior → melhor):")
        for b, avg in ranked:
            n = len(by_blob[b])
            print(f"  {avg:6.1f}  {b}  (n={n})")
    worst = sorted([s for s in scores if s["total_score"] < 50], key=lambda x: x["total_score"])[:top]
    print(f"\nTop {top} piores casos (score < 50):")
    if not worst:
        print("  (nenhum)")
    for s in worst:
        print(f"  {s['total_score']:6.1f}  {s['repo_id']:35s}  {s['blob']}  ({s['size']} bytes)")
    best = sorted([s for s in scores if s["total_score"] >= 80], key=lambda x: -x["total_score"])[:top]
    print(f"\nTop {top} melhores casos (score >= 80):")
    if not best:
        print("  (nenhum)")
    for s in best:
        print(f"  {s['total_score']:6.1f}  {s['repo_id']:35s}  {s['blob']}  ({s['size']} bytes)")


def main() -> int:
    default_root = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(
        description="Auditoria qualitativa 0-100 dos 11 blobs do Harvester."
    )
    parser.add_argument("--db-path", type=Path, default=default_root / ".souls_data" / "souls_heuristic_vault.db")
    parser.add_argument("--out-dir", type=Path, default=default_root / "docs" / "audits" / "quality")
    parser.add_argument("--top", type=int, default=10, help="Top N piores/melhores casos no relatório")
    parser.add_argument("--min-score", type=float, default=50.0, help="Score mínimo para entrar no 'worst cases'")
    parser.add_argument("--repo-allowlist", nargs="*", default=None, help="Filtrar por repo_ids (default: todos)")
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    conn = connect(args.db_path)
    print(f"DB: {args.db_path}")
    rows = fetch_latest(conn, args.repo_allowlist)
    print(f"Pares (repo, blob) coletados: {len(rows)}")
    scores: list[dict[str, Any]] = []
    for r in rows:
        sample = sample_payload(r["payload_blob"])
        s = score_blob(r["repo_id"], r["artifact_type"], int(r["sz"]), sample)
        scores.append(s)
    conn.close()

    # Output
    md = render_markdown(scores, args.top)
    md_path = args.out_dir / "_QUALITY_SCORES.md"
    md_path.write_text(md, encoding="utf-8")
    print(f"\nMarkdown: {md_path}")
    json_path = args.out_dir / "_QUALITY_SCORES.json"
    json_path.write_text(json.dumps(scores, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"JSON:     {json_path}")
    print_summary(scores, args.top)
    return 0


if __name__ == "__main__":
    sys.exit(main())
