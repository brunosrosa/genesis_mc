#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sqlite3
from pathlib import Path
from typing import Iterable


DEFAULT_REPO_IDS = [
    "bytecodealliance/wasmtime",
    "tldraw/tldraw",
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
    root = Path(__file__).resolve().parents[1]
    default_db = root / ".soda_data" / "soda_heuristic_vault.db"
    default_reports = root / "docs" / "audits" / "blobs"

    parser = argparse.ArgumentParser(
        description="Extrai blobs de auditoria da tabela artefatos_brutos para TXT na scratchpad."
    )
    parser.add_argument(
        "--db-path",
        type=Path,
        default=default_db,
        help="Caminho do SQLite do vault heuristico.",
    )
    parser.add_argument(
        "--reports-dir",
        type=Path,
        default=default_reports,
        help="Diretorio de saida dos relatorios TXT.",
    )
    parser.add_argument(
        "--repo-ids",
        nargs="+",
        default=DEFAULT_REPO_IDS,
        help="Lista de repo_ids a extrair.",
    )
    parser.add_argument(
        "--artifact-types",
        nargs="+",
        default=DEFAULT_ARTIFACT_TYPES,
        help="Lista de artifact_types a extrair.",
    )
    parser.add_argument(
        "--max-chars",
        type=int,
        default=0,
        help="Trunca cada payload para N caracteres. Use 0 para nao truncar.",
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


def main() -> int:
    args = parse_args()
    args.reports_dir.mkdir(parents=True, exist_ok=True)

    with connect(args.db_path) as conn:
        payloads, payload_column = fetch_latest_payloads(
            conn,
            repo_ids=args.repo_ids,
            artifact_types=args.artifact_types,
        )

    generated: list[Path] = []
    for artifact_type in args.artifact_types:
        output_path = args.reports_dir / output_name_for(artifact_type)
        report_text = render_report(
            artifact_type=artifact_type,
            repo_ids=args.repo_ids,
            payloads=payloads,
            db_path=args.db_path,
            payload_column=payload_column,
            max_chars=args.max_chars,
        )
        output_path.write_text(report_text, encoding="utf-8", newline="\n")
        generated.append(output_path)

    print("Relatorios gerados:")
    for path in generated:
        print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
