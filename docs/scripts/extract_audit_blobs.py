#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sqlite3
from pathlib import Path
from typing import Iterable


DEFAULT_REPO_IDS = [
    "trailbaseio/trailbase",
]

DEFAULT_ARTIFACT_TYPES = [
    "blob_06_unsafe_hotspots",
    "blob_08_health_report",
]

OUTPUT_NAMES = {
    "blob_06_unsafe_hotspots": "_AUDIT_blob_06_hotspots.txt",
    "blob_08_health_report": "_AUDIT_blob_08_health.txt",
}

CANDIDATE_PAYLOAD_COLUMNS = [
    "conteudo_blob",
    "payload_blob",
    "content_blob",
    "payload",
]


def parse_args() -> argparse.Namespace:
    # parents[0]=scripts, parents[1]=docs, parents[2]=genesis_mc (raiz)
    root = Path(__file__).resolve().parents[2]
    default_db = root / ".soda_data" / "soda_heuristic_vault.db"
    default_reports = root / "docs" / "audits" / "blobs"

    parser = argparse.ArgumentParser(
        description="Extrai blobs de auditoria da tabela artefatos_brutos para TXT na scratchpad. "
                    "Uso geral: --repo-ids e --artifact-types sao listas livres (qualquer blob, qualquer repo)."
    )
    parser.add_argument(
        "--db-path",
        type=Path,
        default=default_db,
        help="Caminho do SQLite do vault heuristico. Default = raiz/.soda_data/soda_heuristic_vault.db",
    )
    parser.add_argument(
        "--reports-dir",
        type=Path,
        default=default_reports,
        help="Diretorio de saida dos relatorios TXT. Default = raiz/docs/audits/blobs",
    )
    parser.add_argument(
        "--repo-ids",
        nargs="+",
        default=None,
        help="Lista de repo_ids a extrair (ex: trailbaseio/trailbase huggingface/candle). Default: so 'trailbaseio/trailbase' (retrocompat).",
    )
    parser.add_argument(
        "--artifact-types",
        nargs="+",
        default=None,
        help="Lista de artifact_types a extrair (ex: blob_06_unsafe_hotspots blob_08_health_report). Default: os 2 do legacy.",
    )
    parser.add_argument(
        "--all-repos",
        action="store_true",
        help="Ignora --repo-ids e varre TODOS os repo_ids distintos do banco (pode ser pesado).",
    )
    parser.add_argument(
        "--all-artifacts",
        action="store_true",
        help="Ignora --artifact-types e extrai TODOS os artifact_types distintos do banco.",
    )
    parser.add_argument(
        "--include-history",
        type=int,
        default=1,
        help="Quantas runs anteriores alem da mais recente incluir no relatorio. 1 = so latest. 0 = latest only. N>1 = mostra diff de tamanhos.",
    )
    parser.add_argument(
        "--max-chars",
        type=int,
        default=0,
        help="Trunca cada payload para N caracteres. Use 0 para nao truncar.",
    )
    parser.add_argument(
        "--pretty-json",
        action="store_true",
        help="Se o payload for JSON, re-formata com indent=2 antes de escrever (silencioso se nao for JSON).",
    )
    parser.add_argument(
        "--summary",
        action="store_true",
        help="Modo dry-run analitico: imprime tabela com (repo, artifact, bytes_latest, ts_latest, n_runs) e sai sem gravar arquivos.",
    )
    parser.add_argument(
        "--no-write",
        action="store_true",
        help="Nao grava arquivos TXT (util combinado com --summary para inspecao rapida).",
    )
    return parser.parse_args()


def connect(db_path: Path) -> sqlite3.Connection:
    if not db_path.exists():
        raise FileNotFoundError(f"SQLite nao encontrado: {db_path}")
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    return conn


def get_table_columns(conn: sqlite3.Connection, table_name: str) -> set[str]:
    rows = conn.execute(f"PRAGMA table_info({table_name})").fetchall()
    if not rows:
        raise RuntimeError(f"Tabela nao encontrada ou vazia no schema: {table_name}")
    return {str(row["name"]) for row in rows}


def detect_payload_column(columns: Iterable[str]) -> str:
    column_set = set(columns)
    for name in CANDIDATE_PAYLOAD_COLUMNS:
        if name in column_set:
            return name
    raise RuntimeError(
        "Nenhuma coluna de payload reconhecida foi encontrada em artefatos_brutos. "
        f"Candidatas testadas: {', '.join(CANDIDATE_PAYLOAD_COLUMNS)}"
    )


def latest_order_clause(columns: set[str]) -> str:
    parts: list[str] = []
    if "timestamp_extracao" in columns:
        parts.append("timestamp_extracao DESC")
    if "artifact_id" in columns:
        parts.append("artifact_id DESC")
    if "id" in columns:
        parts.append("id DESC")
    if not parts:
        parts.append("rowid DESC")
    return ", ".join(parts)


def fetch_latest_payloads(
    conn: sqlite3.Connection,
    repo_ids: list[str],
    artifact_types: list[str],
) -> tuple[dict[tuple[str, str], str], str]:
    columns = get_table_columns(conn, "artefatos_brutos")
    payload_column = detect_payload_column(columns)
    order_clause = latest_order_clause(columns)

    repo_placeholders = ", ".join("?" for _ in repo_ids)
    artifact_placeholders = ", ".join("?" for _ in artifact_types)
    sql = f"""
        WITH ranked AS (
            SELECT
                repo_id,
                artifact_type,
                {payload_column} AS payload,
                ROW_NUMBER() OVER (
                    PARTITION BY repo_id, artifact_type
                    ORDER BY {order_clause}
                ) AS rn
            FROM artefatos_brutos
            WHERE repo_id IN ({repo_placeholders})
              AND artifact_type IN ({artifact_placeholders})
        )
        SELECT repo_id, artifact_type, payload
        FROM ranked
        WHERE rn = 1
        ORDER BY repo_id, artifact_type
    """
    rows = conn.execute(sql, [*repo_ids, *artifact_types]).fetchall()

    out: dict[tuple[str, str], str] = {}
    for row in rows:
        raw_payload = row["payload"]
        if isinstance(raw_payload, bytes):
            text = raw_payload.decode("utf-8", errors="replace")
        elif raw_payload is None:
            text = ""
        else:
            text = str(raw_payload)
        out[(str(row["repo_id"]), str(row["artifact_type"]))] = text

    return out, payload_column


def render_report(
    artifact_type: str,
    repo_ids: list[str],
    payloads: dict[tuple[str, str], str],
    db_path: Path,
    payload_column: str,
    max_chars: int,
) -> str:
    lines = [
        f"# AUDIT REPORT: {artifact_type}",
        f"# DB: {db_path}",
        f"# PAYLOAD_COLUMN: {payload_column}",
        "",
    ]

    for repo_id in repo_ids:
        lines.append(f"=== REPO: {repo_id} ===")
        payload = payloads.get((repo_id, artifact_type))
        if payload is None:
            lines.append("[sem payload encontrado]")
        else:
            text = payload if max_chars <= 0 else payload[:max_chars]
            lines.append(text.rstrip() if text else "[payload vazio]")
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def output_name_for(artifact_type: str) -> str:
    if artifact_type in OUTPUT_NAMES:
        return OUTPUT_NAMES[artifact_type]
    sanitized = artifact_type.replace("/", "_").replace(" ", "_")
    return f"_AUDIT_{sanitized}.txt"


def maybe_pretty(text: str, enabled: bool) -> str:
    """Re-formata JSON se habilitado. Silencioso se nao for JSON valido."""
    if not enabled or not text:
        return text
    try:
        import json
        parsed = json.loads(text)
        return json.dumps(parsed, indent=2, ensure_ascii=False)
    except (ValueError, TypeError):
        return text


def list_distinct(conn: sqlite3.Connection, column: str) -> list[str]:
    """Lista valores distintos de uma coluna (uso em --all-repos/--all-artifacts)."""
    quoted = '"' + column.replace('"', '""') + '"'
    rows = conn.execute(f"SELECT DISTINCT {quoted} FROM artefatos_brutos ORDER BY {quoted}").fetchall()
    return [str(r[0]) for r in rows if r[0] is not None]


def resolve_targets(conn: sqlite3.Connection, args: argparse.Namespace) -> tuple[list[str], list[str]]:
    """Resolve --all-repos/--all-artifacts OU cai nos defaults legacy."""
    repo_ids = args.repo_ids if not args.all_repos else list_distinct(conn, "repo_id")
    if not repo_ids:
        repo_ids = list(DEFAULT_REPO_IDS)
    artifact_types = args.artifact_types if not args.all_artifacts else list_distinct(conn, "artifact_type")
    if not artifact_types:
        artifact_types = list(DEFAULT_ARTIFACT_TYPES)
    return repo_ids, artifact_types


def print_summary(
    conn: sqlite3.Connection,
    repo_ids: list[str],
    artifact_types: list[str],
    payload_column: str,
) -> None:
    """Tabela analitica: (repo, artifact, bytes_latest, ts_latest_iso, n_runs_total)."""
    print(f"{'repo_id':<32s} {'artifact_type':<32s} {'bytes':>10s} {'timestamp_iso':<26s} {'runs':>6s}")
    print("-" * 110)
    for repo_id in repo_ids:
        for artifact_type in artifact_types:
            cur = conn.execute(
                f"""
                SELECT
                    length({payload_column}) AS sz,
                    timestamp_extracao AS ts,
                    (SELECT count(*) FROM artefatos_brutos ab2
                     WHERE ab2.repo_id=ab.repo_id AND ab2.artifact_type=ab.artifact_type) AS n
                FROM artefatos_brutos ab
                WHERE repo_id=? AND artifact_type=?
                ORDER BY timestamp_extracao DESC LIMIT 1
                """,
                (repo_id, artifact_type),
            )
            row = cur.fetchone()
            if not row:
                print(f"{repo_id:<32s} {artifact_type:<32s} {'(none)':>10s} {'-':<26s} {'-':>6s}")
                continue
            ts_iso = ""
            if row["ts"]:
                try:
                    import datetime as _dt
                    ts_iso = _dt.datetime.fromtimestamp(int(row["ts"])).isoformat(timespec="seconds")
                except Exception:
                    ts_iso = str(row["ts"])
            print(f"{repo_id:<32s} {artifact_type:<32s} {row['sz']:>10d} {ts_iso:<26s} {row['n']:>6d}")
    print("-" * 110)


def fetch_history(
    conn: sqlite3.Connection,
    repo_id: str,
    artifact_type: str,
    payload_column: str,
    n: int,
) -> list[dict]:
    """Ultimas N runs (incluindo a latest) com (ts, bytes)."""
    rows = conn.execute(
        f"""
        SELECT timestamp_extracao AS ts, length({payload_column}) AS sz
        FROM artefatos_brutos
        WHERE repo_id=? AND artifact_type=?
        ORDER BY timestamp_extracao DESC LIMIT ?
        """,
        (repo_id, artifact_type, max(1, n)),
    ).fetchall()
    return [{"ts": int(r["ts"]), "sz": int(r["sz"])} for r in rows]


def render_report(
    artifact_type: str,
    repo_ids: list[str],
    payloads: dict[tuple[str, str], str],
    db_path: Path,
    payload_column: str,
    max_chars: int,
    pretty_json: bool,
    history: dict[tuple[str, str], list[dict]] | None = None,
) -> str:
    lines = [
        f"# AUDIT REPORT: {artifact_type}",
        f"# DB: {db_path}",
        f"# PAYLOAD_COLUMN: {payload_column}",
        f"# GENERATED_AT: {__import__('datetime').datetime.now().isoformat(timespec='seconds')}",
        "",
    ]

    for repo_id in repo_ids:
        lines.append(f"=== REPO: {repo_id} ===")
        if history is not None and (repo_id, artifact_type) in history:
            hist = history[(repo_id, artifact_type)]
            if hist:
                latest = hist[0]
                lines.append(f"# history_runs={len(hist)}  latest_bytes={latest['sz']}  latest_ts={latest['ts']}")
                if len(hist) > 1:
                    deltas = [f"{h['sz'] - latest['sz']:+d}" for h in hist[1:]]
                    lines.append(f"# history_deltas_bytes={deltas}")
        payload = payloads.get((repo_id, artifact_type))
        if payload is None:
            lines.append("[sem payload encontrado]")
        else:
            text = maybe_pretty(payload, pretty_json)
            text = text if max_chars <= 0 else text[:max_chars]
            lines.append(text.rstrip() if text else "[payload vazio]")
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    args = parse_args()

    if not args.db_path.exists():
        raise FileNotFoundError(f"SQLite nao encontrado: {args.db_path}")

    with connect(args.db_path) as conn:
        repo_ids, artifact_types = resolve_targets(conn, args)
        payload_column = detect_payload_column(get_table_columns(conn, "artefatos_brutos"))

        if args.summary or args.no_write:
            print(f"DB: {args.db_path}")
            print(f"PAYLOAD_COLUMN: {payload_column}")
            print(f"REPOS ({len(repo_ids)}): {len(repo_ids)}  ARTIFACTS ({len(artifact_types)}): {len(artifact_types)}")
            print()
            print_summary(conn, repo_ids, artifact_types, payload_column)
            return 0

        payloads, _ = fetch_latest_payloads(
            conn,
            repo_ids=repo_ids,
            artifact_types=artifact_types,
        )

        # History opcional para diagnostico de delta de tamanho
        history: dict[tuple[str, str], list[dict]] = {}
        if args.include_history > 1:
            for r in repo_ids:
                for a in artifact_types:
                    history[(r, a)] = fetch_history(conn, r, a, payload_column, args.include_history)

    args.reports_dir.mkdir(parents=True, exist_ok=True)

    generated: list[Path] = []
    for artifact_type in artifact_types:
        output_path = args.reports_dir / output_name_for(artifact_type)
        report_text = render_report(
            artifact_type=artifact_type,
            repo_ids=repo_ids,
            payloads=payloads,
            db_path=args.db_path,
            payload_column=payload_column,
            max_chars=args.max_chars,
            pretty_json=args.pretty_json,
            history=history if history else None,
        )
        output_path.write_text(report_text, encoding="utf-8", newline="\n")
        generated.append(output_path)

    print("Relatorios gerados:")
    for path in generated:
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
