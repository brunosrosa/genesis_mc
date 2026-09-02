#!/usr/bin/env python3
"""
Teste de smoke do `audit_workspace_compliance.py`.

Valida que:
  1. Script exit 0 quando workspace OK (com --quiet)
  2. Script exit != 0 quando detecta ERROR
  3. Script detecta refs antigas conhecidas
  4. Script detecta paths não-canônicos
  5. Saída JSON é bem-formada

Uso:
  python test_audit_workspace_compliance.py
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path


SCRIPT_PATH = Path(__file__).resolve().parent / "audit_workspace_compliance.py"


def run_script(*args: str, workspace: Path | None = None) -> subprocess.CompletedProcess:
    cmd = [sys.executable, str(SCRIPT_PATH), *args]
    return subprocess.run(
        cmd,
        cwd=workspace or Path(__file__).resolve().parent.parent.parent.parent,
        capture_output=True,
        text=True,
        # Em Windows, sem shell intermediário
        shell=False,
    )


def test_workspace_clean_baseline():
    """1. Em workspace normal, --quiet deve retornar exit 0."""
    result = run_script("--quiet")
    print(f"  [1] exit_code={result.returncode} (esperado 0)")
    assert result.returncode == 0, f"Esperado exit 0, obtido {result.returncode}\n{result.stdout}\n{result.stderr}"
    print("  [1] OK: workspace baseline limpo")


def test_json_output_is_well_formed():
    """2. Saída --json deve ser JSON bem-formado com campos esperados."""
    result = run_script("--json", "--quiet")
    assert result.returncode == 0, f"Exit != 0: {result.stderr}"
    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        print(f"  [2] FALHOU: JSON inválido: {e}\n{result.stdout[:200]}")
        sys.exit(1)
    assert "workspace" in data, "Campo 'workspace' ausente"
    assert "map_version" in data, "Campo 'map_version' ausente"
    assert "findings" in data, "Campo 'findings' ausente"
    assert "summary" in data, "Campo 'summary' ausente"
    print(f"  [2] OK: JSON bem-formado (workspace={data['workspace']}, map_version={data['map_version']})")


def test_deprecated_refs_in_temp_workspace():
    """3. Em workspace temporário com ref antiga conhecida, deve detectar."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Criar estrutura mínima que satisfaça as zonas obrigatórias
        for zone in [
            "docs/work-units/", "docs/planning/", "docs/decisions/",
            "docs/observability/", "docs/runtime/", "docs/debugs/",
            "docs/SOULS_CANON_MANIFEST.md",
        ]:
            (Path(tmpdir) / zone).mkdir(parents=True, exist_ok=True)
        # Criar _WORKSPACE_MAP.md com version: 9.9
        (Path(tmpdir) / "_WORKSPACE_MAP.md").write_text("---\nversion: 9.9\n---\n", encoding="utf-8")
        # Criar arquivo com ref antiga (montada dinamicamente para evitar
        # que o proprio arquivo de teste seja detectado pelo auditor)
        deprecated_path = "docs/" + "adrs" + "/ADR-001-foo.md"
        (Path(tmpdir) / "docs/decisions/test.md").write_text(
            f"Link quebrado: [ADR-001]({deprecated_path})\n", encoding="utf-8"
        )
        result = run_script("--json", workspace=Path(tmpdir))
        # Como há 1 ref antiga, deve ter pelo menos 1 finding
        data = json.loads(result.stdout)
        deprecated_findings = [
            f for f in data["findings"] if f["category"] == "deprecated_ref"
        ]
        assert len(deprecated_findings) >= 1, (
            f"Esperado >=1 ref antiga detectada, obtido {len(deprecated_findings)}\n"
            f"Output: {result.stdout[:500]}"
        )
        print(f"  [3] OK: detectou {len(deprecated_findings)} ref(s) antiga(s) em workspace de teste")


def test_missing_required_zone_detected():
    """4. Em workspace sem zona obrigatória, deve detectar ERROR."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Criar APENAS _WORKSPACE_MAP.md, sem zonas
        (Path(tmpdir) / "_WORKSPACE_MAP.md").write_text("---\nversion: 9.9\n---\n", encoding="utf-8")
        result = run_script("--json", workspace=Path(tmpdir))
        data = json.loads(result.stdout)
        missing_findings = [
            f for f in data["findings"] if f["category"] == "missing_required_zone"
        ]
        assert len(missing_findings) >= 1, (
            f"Esperado >=1 zona faltando detectada, obtido {len(missing_findings)}"
        )
        # Exit code deve ser != 0 (ERROR presente)
        assert result.returncode != 0, "Exit code deveria ser != 0 com ERROR"
        print(f"  [4] OK: detectou {len(missing_findings)} zona(s) faltando, exit={result.returncode}")


def test_non_canonical_path_detected():
    """5. Arquivo tracked em path não-canônico deve ser detectado."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Criar zonas obrigatórias
        for zone in [
            "docs/work-units/", "docs/planning/", "docs/decisions/",
            "docs/observability/", "docs/runtime/", "docs/debugs/",
            "docs/SOULS_CANON_MANIFEST.md",
        ]:
            (Path(tmpdir) / zone).mkdir(parents=True, exist_ok=True)
        (Path(tmpdir) / "_WORKSPACE_MAP.md").write_text("---\nversion: 9.9\n---\n", encoding="utf-8")
        # Criar arquivo em path suspeito (gitignored mas tracked)
        (Path(tmpdir) / "random_uncanonical_file.xyz").write_text("x", encoding="utf-8")
        # Inicializar git + add + commit
        subprocess.run(["git", "init", "-q"], cwd=tmpdir, check=True)
        subprocess.run(["git", "add", "-A"], cwd=tmpdir, check=True)
        subprocess.run(
            ["git", "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "init"],
            cwd=tmpdir, check=True,
        )
        result = run_script("--json", workspace=Path(tmpdir))
        data = json.loads(result.stdout)
        non_canonical = [
            f for f in data["findings"] if f["category"] == "non_canonical_path"
        ]
        assert len(non_canonical) >= 1, (
            f"Esperado >=1 path não-canônico, obtido {len(non_canonical)}\n"
            f"Findings: {data['findings']}"
        )
        print(f"  [5] OK: detectou {len(non_canonical)} path(s) não-canônico(s)")


def main() -> int:
    print("=" * 60)
    print("  Smoke tests: audit_workspace_compliance.py")
    print("=" * 60)
    tests = [
        test_workspace_clean_baseline,
        test_json_output_is_well_formed,
        test_deprecated_refs_in_temp_workspace,
        test_missing_required_zone_detected,
        test_non_canonical_path_detected,
    ]
    for t in tests:
        print(f"\n{t.__doc__.splitlines()[0]}")
        try:
            t()
        except AssertionError as e:
            print(f"  FALHOU: {e}")
            return 1
        except Exception as e:
            print(f"  ERRO INESPERADO: {type(e).__name__}: {e}")
            return 1
    print("\n" + "=" * 60)
    print("  TODOS OS TESTES PASSARAM")
    print("=" * 60)
    return 0


if __name__ == "__main__":
    sys.exit(main())
