#!/usr/bin/env python3
"""
Souls Territorial Compliance Auditor (Marco 3.10 — F7)

Detecta violações territoriais no workspace SOULS:
  1. Arquivos .md/.ps1/.py/... em paths não-canônicos (fora das zonas oficiais)
  2. Refs a paths antigos (docs/prds/, docs/adrs/, etc.) que foram movidos no Marco 3.10
  3. Pastas tracked que estão em zonas gitignored (.archive/, .souls_data/, etc.)

Uso:
  python audit_workspace_compliance.py                 # audita tudo
  python audit_workspace_compliance.py --json         # saída JSON
  python audit_workspace_compliance.py --staged-only  # só o que está staged
  python audit_workspace_compliance.py --quiet        # exit 0 mesmo com findings

Exit code:
  0 = sem violações (ou --quiet)
  1 = violações detectadas
  2 = erro de execução (workspace inválido)

Referência: docs/SOULS_CANON_MANIFEST.md, _WORKSPACE_MAP.md v6.0
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Iterable

# ------------------------------------------------------------------------------
# CONFIGURAÇÃO CANÔNICA
# ------------------------------------------------------------------------------

WORKSPACE_ROOT = Path(__file__).resolve().parent.parent.parent.parent
MAP_FILE = WORKSPACE_ROOT / "_WORKSPACE_MAP.md"

# Pastas/Zonas que são CANÔNICAS (a presença aqui é OK)
CANONICAL_ZONES: dict[str, list[str]] = {
    "ZONA 1 - Fábrica & Agente (gitignored)": [
        ".agents/", ".trae/", ".antigravitycli/", ".vscode/",
    ],
    "ZONA 2 - Estado & Cache (gitignored)": [
        ".souls_data/", ".souls_cache/", ".souls_sandbox/", ".souls_scratchpad/",
        ".souls/",
    ],
    "ZONA 3 - Cânone (raiz)": [
        "docs/",
        ".archive/",
    ],
    "ZONA 4 - Backend Rust": [
        "src-tauri/Cargo.toml", "src-tauri/Cargo.lock", "src-tauri/build.rs",
        "src-tauri/tauri.conf.json", "src-tauri/src/", "src-tauri/third_party/",
        "src-tauri/icons/", "src-tauri/.gitignore", "src-tauri/vendor/",
        "src-tauri/capabilities/", "src-tauri/examples/",
        "src-tauri/resources/", "src-tauri/semgrep/",
        "src-tauri/crates/", "src-tauri/tests/", "src-tauri/scripts/",
        "src-tauri/data/", "src-tauri/bin/", "src-tauri/gen/",
        "src-tauri/.cargo/",
    ],
    "ZONA 5 - Frontend Svelte": [
        "src/components/", "src/lib/", "src/routes/",
        "src/app.html", "src/app.css", "src/main.ts", "src/app.d.ts",
        "src/vite-env.d.ts", "src/style.css",
        "src/App.svelte", "src/index.css",
    ],
    "ZONA 6 - Raiz (arquivos pontuais)": [
        "README.md", "LICENSE", "Cargo.toml", "Cargo.lock", "package.json",
        "pnpm-lock.yaml", "pnpm-workspace.yaml", ".npmrc", ".gitmodules",
        "tsconfig.json", "tsconfig.node.json",
        "svelte.config.js", "vite.config.ts",
        "tailwind.config.js", "postcss.config.js", ".gitignore",
        ".gitattributes", ".editorconfig", ".prettierrc", ".eslintrc.cjs",
        "boot.ps1", "boot.sh", "_WORKSPACE_MAP.md",
        # Marcos 3.10 — paths adicionais da raiz
        ".cargo/", ".env.example", "GEMINI.md",
        "app-icon.png", "components.json", "gateway-config.yaml",
        "index.html", "schema.json",
    ],
}

# Path-prefixes que DEVEM existir (zonas obrigatórias)
REQUIRED_ZONE_PREFIXES: list[str] = [
    "docs/work-units/",
    "docs/planning/",
    "docs/decisions/",
    "docs/observability/",
    "docs/runtime/",
    "docs/debugs/",
    "docs/SOULS_CANON_MANIFEST.md",
]

# Paths ANTIGOS que foram migrados no Marco 3.10 — sua presença em refs
# indica território quebrado que precisa ser consertado.
DEPRECATED_PATH_PATTERNS: list[tuple[str, str, str]] = [
    # (regex, replacement_text, descricao)
    (r"\bdocs/prds/", "docs/planning/prds/", "docs/prds/ -> docs/planning/prds/"),
    (r"\bdocs/adrs/", "docs/decisions/adrs/", "docs/adrs/ -> docs/decisions/adrs/"),
    (r"\bdocs/architecture/manifesto", "docs/decisions/architecture/manifesto", "docs/architecture/manifesto"),
    (r"\bdocs/architecture/core_daemon", "docs/decisions/architecture/core_daemon", "docs/architecture/core_daemon"),
    (r"\bdocs/architecture/gateway_routing", "docs/decisions/architecture/gateway_routing", "docs/architecture/gateway_routing"),
    (r"\bdocs/architecture/governance_topology", "docs/decisions/architecture/governance_topology", "docs/architecture/governance_topology"),
    (r"\bdocs/architecture/inference_engine", "docs/decisions/architecture/inference_engine", "docs/architecture/inference_engine"),
    (r"\bdocs/architecture/memory_system", "docs/decisions/architecture/memory_system", "docs/architecture/memory_system"),
    (r"\bdocs/architecture/essence-post-mortem", "docs/decisions/architecture/canibalization_essence/essence-post-mortem", "docs/architecture/essence-post-mortem"),
    (r"\bdocs/specs/spec-", "docs/decisions/specs/spec-", "docs/specs/spec- -> docs/decisions/specs/spec-"),
    (r"\bdocs/audits/", "docs/observability/audits/", "docs/audits/ -> docs/observability/audits/"),
    (r"\bdocs/state/", "docs/observability/state/", "docs/state/ -> docs/observability/state/"),
    (r"\bdocs/reports/", "docs/observability/reports/", "docs/reports/ -> docs/observability/reports/"),
    (r"\bdocs/dags/", "docs/runtime/dags/", "docs/dags/ -> docs/runtime/dags/"),
    (r"\bdocs/context_dumps/", "docs/runtime/context_dumps/", "docs/context_dumps/ -> docs/runtime/context_dumps/"),
    (r"\bdocs/scripts/", "docs/runtime/scripts/", "docs/scripts/ -> docs/runtime/scripts/"),
    (r"\bdocs/milestones/", "docs/planning/roadmap/", "docs/milestones/ -> docs/planning/roadmap/"),
    (r"\bdocs/fixes/", "docs/work-units/active/", "docs/fixes/ -> docs/work-units/active/"),
    (r"\bdocs/features/", "docs/work-units/active/", "docs/features/ -> docs/work-units/active/"),
]

# Extensões de arquivos onde refs quebradas importam
SCANNABLE_EXTENSIONS: set[str] = {
    ".md", ".ps1", ".py", ".psm1", ".psd1", ".json", ".yaml", ".yml",
    ".toml", ".cfg", ".ini", ".conf", ".sh", ".bash", ".zsh", ".fish",
    ".rs", ".ts", ".js", ".svelte", ".html", ".css", ".txt", ".csv",
}

# Pastas a ignorar COMPLETAMENTE durante a varredura
IGNORED_DIRS: set[str] = {
    ".git", "node_modules", "target", "dist", "build", "vendor",
    ".archive", ".souls_data", ".souls_cache", ".souls_sandbox",
    ".souls_scratchpad", ".pytest_cache", ".antigravitycli",
    ".agents", ".trae", ".vscode", ".lean-ctx", ".jcodemunch_index",
}

# Arquivos individuais a ignorar (frozen artifacts que preservam histórico)
# Marco 3.10 — frozen audits blob_04/blob_05 e context_dumps/_*.txt
# contêm refs antigas preservadas intencionalmente como evidência
# forense de quando o repo tinha a topologia antiga. NÃO atualizar
# (são snapshots históricos gerados pelo harvester e pelo
# souls_context_dumps_compiler.py).
IGNORED_FILES: set[str] = {
    "docs/observability/audits/blobs/_AUDIT_blob_04_repo_outline.txt",
    "docs/observability/audits/blobs/_AUDIT_blob_05_architecture_map.txt",
    "docs/runtime/context_dumps/_ADRs_ALL.txt",
    "docs/runtime/context_dumps/_RULES_IN_IDEs.txt",
    "docs/runtime/context_dumps/_SKILLS_IN_IDEs.txt",
    "docs/runtime/context_dumps/_ENV_CLEAN.txt",
    "docs/runtime/context_dumps/_IGNITION_SCRIPTS.txt",
    "docs/runtime/context_dumps/_MCPS_LIST.txt",
    "docs/runtime/context_dumps/_MCP_INVENTORY.txt",
    "docs/runtime/context_dumps/_MODELS_INVENTORY.txt",
    "docs/runtime/context_dumps/_WOKSPACE_MAP.txt",
}


# ------------------------------------------------------------------------------
# MODELO DE ACHADOS
# ------------------------------------------------------------------------------

@dataclass
class Finding:
    """Achado de auditoria territorial."""
    severity: str  # "ERROR" | "WARN" | "INFO"
    category: str  # "deprecated_ref" | "non_canonical_path" | "missing_required_zone"
    path: str
    line: int | None
    message: str
    fix_hint: str = ""


@dataclass
class AuditReport:
    """Relatório de auditoria territorial."""
    workspace: str
    map_version: str
    findings: list[Finding] = field(default_factory=list)
    scanned_files: int = 0
    canonical_zones_ok: int = 0
    deprecated_refs_found: int = 0
    non_canonical_paths_found: int = 0

    def to_dict(self) -> dict:
        return {
            "workspace": self.workspace,
            "map_version": self.map_version,
            "summary": {
                "scanned_files": self.scanned_files,
                "canonical_zones_ok": self.canonical_zones_ok,
                "deprecated_refs_found": self.deprecated_refs_found,
                "non_canonical_paths_found": self.non_canonical_paths_found,
                "total_findings": len(self.findings),
                "by_severity": {
                    "ERROR": sum(1 for f in self.findings if f.severity == "ERROR"),
                    "WARN": sum(1 for f in self.findings if f.severity == "WARN"),
                    "INFO": sum(1 for f in self.findings if f.severity == "INFO"),
                },
            },
            "findings": [asdict(f) for f in self.findings],
        }


# ------------------------------------------------------------------------------
# DETECÇÃO DO WORKSPACE
# ------------------------------------------------------------------------------

def detect_workspace() -> Path:
    """Detecta raiz do workspace subindo até encontrar _WORKSPACE_MAP.md.

    Estratégia (em ordem):
    1. Começa pelo `cwd` (cwd do processo).
    2. Sobe até 8 níveis procurando _WORKSPACE_MAP.md.
    3. Fallback: começa pelo diretório do __file__ (modo invocação direta).
    """
    # Tenta primeiro a partir do cwd
    cur = Path.cwd().resolve()
    for _ in range(8):
        if (cur / "_WORKSPACE_MAP.md").is_file():
            return cur
        if cur.parent == cur:
            break
        cur = cur.parent
    # Fallback: __file__ do script
    cur = Path(__file__).resolve().parent
    for _ in range(8):
        if (cur / "_WORKSPACE_MAP.md").is_file():
            return cur
        if cur.parent == cur:
            break
        cur = cur.parent
    raise RuntimeError(
        "Não foi possível localizar _WORKSPACE_MAP.md. "
        "Execute o script a partir do workspace SOULS."
    )


def parse_map_version() -> str:
    """Extrai `version: X.Y` do frontmatter YAML do _WORKSPACE_MAP.md."""
    if not MAP_FILE.is_file():
        return "unknown"
    text = MAP_FILE.read_text(encoding="utf-8", errors="replace")
    m = re.search(r"^version:\s*([0-9.]+)", text, re.MULTILINE)
    return m.group(1) if m else "unknown"


# ------------------------------------------------------------------------------
# VERIFICAÇÃO DE ZONAS CANÔNICAS
# ------------------------------------------------------------------------------

def check_required_zones(report: AuditReport) -> None:
    """Verifica que cada zona canônica obrigatória existe no disco."""
    for prefix in REQUIRED_ZONE_PREFIXES:
        full = WORKSPACE_ROOT / prefix
        # Prefixes terminados em '/' DEVEM ser diretórios.
        # Prefixes sem '/' DEVEM ser arquivos (ex: SOULS_CANON_MANIFEST.md).
        expected_kind = "dir" if prefix.endswith("/") else "file"
        ok = (
            full.is_dir() if expected_kind == "dir"
            else full.is_file()
        )
        if not ok:
            report.findings.append(Finding(
                severity="ERROR",
                category="missing_required_zone",
                path=prefix,
                line=None,
                message=f"Zona obrigatória ausente: {prefix} (esperado: {expected_kind})",
                fix_hint=f"Reorganize o workspace para criar {prefix} (ver _WORKSPACE_MAP.md v{report.map_version}).",
            ))
        else:
            report.canonical_zones_ok += 1


# ------------------------------------------------------------------------------
# DETECÇÃO DE REFS ANTIGAS
# ------------------------------------------------------------------------------

def iter_scannable_files(workspace: Path) -> Iterable[Path]:
    """Itera sobre todos os arquivos scannable do workspace, ignorando zonas proibidas."""
    for dirpath, dirnames, filenames in os.walk(workspace):
        # Filtra dirs ignorados in-place para não recursar
        dirnames[:] = [d for d in dirnames if d not in IGNORED_DIRS and not d.startswith("target")]
        for name in filenames:
            p = Path(dirpath) / name
            if p.suffix.lower() in SCANNABLE_EXTENSIONS:
                yield p


def scan_deprecated_refs(workspace: Path, report: AuditReport) -> None:
    """Varre arquivos procurando refs a paths antigos do Marco 3.10."""
    compiled = [(re.compile(p), r, d) for p, r, d in DEPRECATED_PATH_PATTERNS]
    for f in iter_scannable_files(workspace):
        # Ignora frozen artifacts preservados intencionalmente
        try:
            rel = str(f.relative_to(workspace)).replace("\\", "/")
        except ValueError:
            continue
        if rel in IGNORED_FILES:
            continue
        # Ignora o próprio script (ele contém os regex como strings)
        if rel == "docs/runtime/scripts/audit_workspace_compliance.py":
            continue
        try:
            text = f.read_text(encoding="utf-8", errors="replace")
        except (OSError, PermissionError):
            continue
        report.scanned_files += 1
        for lineno, line in enumerate(text.splitlines(), start=1):
            for rx, replacement, desc in compiled:
                if rx.search(line):
                    report.findings.append(Finding(
                        severity="WARN",
                        category="deprecated_ref",
                        path=str(f.relative_to(workspace)),
                        line=lineno,
                        message=f"Ref antiga detectada: {desc}",
                        fix_hint=f"Substituir por: {replacement}",
                    ))
                    report.deprecated_refs_found += 1


# ------------------------------------------------------------------------------
# DETECÇÃO DE PATHS NÃO-CANÔNICOS
# ------------------------------------------------------------------------------

def is_canonical(path_rel: str) -> bool:
    """Verifica se um path relativo está dentro de alguma zona canônica."""
    for prefixes in CANONICAL_ZONES.values():
        for prefix in prefixes:
            if prefix.endswith("/") and path_rel.startswith(prefix):
                return True
            if not prefix.endswith("/") and (path_rel == prefix or path_rel.startswith(prefix + "/")):
                return True
    return False


def scan_non_canonical_paths(workspace: Path, report: AuditReport) -> None:
    """Detecta arquivos tracked em paths não-canônicos."""
    try:
        result = subprocess.run(
            ["git", "ls-files"],
            cwd=workspace,
            capture_output=True,
            text=True,
            check=True,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        # Sem git ou repo quebrado: pula silenciosamente
        return
    for line in result.stdout.splitlines():
        path_rel = line.strip()
        # git ls-files no Windows pode retornar paths com aspas duplas
        # quando há caracteres especiais (acentos, espaços, colchetes).
        if path_rel.startswith('"') and path_rel.endswith('"'):
            path_rel = path_rel[1:-1]
        if not path_rel or path_rel.startswith(".git/"):
            continue
        if not is_canonical(path_rel):
            report.findings.append(Finding(
                severity="WARN",
                category="non_canonical_path",
                path=path_rel,
                line=None,
                message=f"Arquivo tracked em path não-canônico: {path_rel}",
                fix_hint="Mova para uma zona canônica (ver _WORKSPACE_MAP.md v6.0).",
            ))
            report.non_canonical_paths_found += 1


# ------------------------------------------------------------------------------
# SAÍDA
# ------------------------------------------------------------------------------

def render_text(report: AuditReport) -> str:
    lines: list[str] = []
    lines.append("=" * 78)
    lines.append(f" SOULS TERRITORIAL COMPLIANCE AUDITOR — Marco 3.10 (F7)")
    lines.append(f" Workspace: {report.workspace}")
    lines.append(f" _WORKSPACE_MAP.md version: {report.map_version}")
    lines.append("=" * 78)
    summary = report.to_dict()["summary"]
    lines.append("")
    lines.append("SUMMARY")
    lines.append(f"  Arquivos scaneados     : {summary['scanned_files']}")
    lines.append(f"  Zonas canonicas OK     : {summary['canonical_zones_ok']}/{len(REQUIRED_ZONE_PREFIXES)}")
    lines.append(f"  Refs antigas detectadas: {summary['deprecated_refs_found']}")
    lines.append(f"  Paths nao-canonicos    : {summary['non_canonical_paths_found']}")
    lines.append(f"  Total de findings      : {summary['total_findings']}")
    lines.append(f"  Por severidade         : ERROR={summary['by_severity']['ERROR']} "
                 f"WARN={summary['by_severity']['WARN']} "
                 f"INFO={summary['by_severity']['INFO']}")
    lines.append("")
    if not report.findings:
        lines.append("[OK] Nenhuma violação territorial detectada.")
        return "\n".join(lines)
    lines.append("FINDINGS")
    lines.append("-" * 78)
    for i, f in enumerate(report.findings, start=1):
        loc = f"{f.path}" + (f":{f.line}" if f.line else "")
        lines.append(f"[{i:03d}] {f.severity} {f.category}")
        lines.append(f"      LOCATION: {loc}")
        lines.append(f"      MESSAGE : {f.message}")
        if f.fix_hint:
            lines.append(f"      FIX     : {f.fix_hint}")
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Souls Territorial Compliance Auditor (Marco 3.10 F7)"
    )
    parser.add_argument("--json", action="store_true", help="Saída em JSON")
    parser.add_argument("--quiet", action="store_true", help="Exit 0 mesmo com findings")
    parser.add_argument("--no-refs-scan", action="store_true", help="Pular varredura de refs")
    parser.add_argument("--no-paths-scan", action="store_true", help="Pular varredura de paths")
    parser.add_argument("--no-zones-check", action="store_true", help="Pular check de zonas obrigatórias")
    args = parser.parse_args()

    try:
        global WORKSPACE_ROOT
        WORKSPACE_ROOT = detect_workspace()
    except RuntimeError as e:
        print(f"ERRO: {e}", file=sys.stderr)
        return 2

    report = AuditReport(
        workspace=str(WORKSPACE_ROOT),
        map_version=parse_map_version(),
    )

    if not args.no_zones_check:
        check_required_zones(report)
    if not args.no_refs_scan:
        scan_deprecated_refs(WORKSPACE_ROOT, report)
    if not args.no_paths_scan:
        scan_non_canonical_paths(WORKSPACE_ROOT, report)

    if args.json:
        print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
    else:
        print(render_text(report))

    has_errors = any(f.severity == "ERROR" for f in report.findings)
    if args.quiet:
        return 0
    return 1 if has_errors else 0


if __name__ == "__main__":
    sys.exit(main())
