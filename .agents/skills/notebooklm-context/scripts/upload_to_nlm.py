"""
Upload em lote dos temas cristalizados ao NotebookLM V2.
Usa 'nlm add text' para injetar cada arquivo como fonte de texto.
"""
import os
import subprocess
import sys
import time
from pathlib import Path

NOTEBOOK_ID = "0737996f-cf30-4050-a9a8-e18a16899937"
CRYSTALIZED_DIR = Path(__file__).parent / "soda_canon" / "crystalized"

# Limite seguro para argumento de linha de comando no Windows (~30K chars)
# Arquivos maiores serao divididos em partes
MAX_CLI_ARG_CHARS = 28000


def upload_text(notebook_id: str, title: str, content: str) -> bool:
    """Envia texto como fonte ao NotebookLM via CLI."""
    cmd = [
        "nlm", "add", "text",
        notebook_id,
        content,
        "--title", title,
        "--wait",
        "--wait-timeout", "120",
    ]
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
            timeout=180,
        )
        if result.returncode == 0:
            return True
        else:
            err = (result.stderr or result.stdout or "")[:300]
            print(f"  ERRO: {err}")
            return False
    except subprocess.TimeoutExpired:
        print("  ERRO: Timeout de 3 minutos.")
        return False
    except Exception as e:
        print(f"  ERRO: {e}")
        return False


def main():
    files = sorted(CRYSTALIZED_DIR.glob("Theme_*_canonical.md"))
    total = len(files)

    print(f"\n{'='*60}")
    print(f"  Upload SODA Canon V2 ao NotebookLM")
    print(f"  Notebook: {NOTEBOOK_ID}")
    print(f"  Arquivos: {total}")
    print(f"{'='*60}\n")

    successes = 0
    failures = []

    for idx, f in enumerate(files, 1):
        theme_name = f.stem.replace("_canonical", "")  # ex: Theme_01
        size_kb = f.stat().st_size // 1024

        with open(f, "r", encoding="utf-8") as fh:
            content = fh.read()

        # Se o conteudo for maior que o limite de argumento CLI, divide em partes
        if len(content) > MAX_CLI_ARG_CHARS:
            parts = [content[i:i+MAX_CLI_ARG_CHARS] for i in range(0, len(content), MAX_CLI_ARG_CHARS)]
            print(f"  [{idx}/{total}] {theme_name} ({size_kb}KB) -> {len(parts)} partes...", flush=True)

            all_ok = True
            for p_idx, part in enumerate(parts):
                part_title = f"SODA {theme_name} (Parte {p_idx+1}/{len(parts)})"
                print(f"    Parte {p_idx+1}/{len(parts)}...", end=" ", flush=True)
                ok = upload_text(NOTEBOOK_ID, part_title, part)
                if ok:
                    print("OK")
                else:
                    print("FALHA")
                    all_ok = False
                time.sleep(4)

            if all_ok:
                successes += 1
            else:
                failures.append(theme_name)
        else:
            title = f"SODA {theme_name}"
            print(f"  [{idx}/{total}] {theme_name} ({size_kb}KB)...", end=" ", flush=True)
            ok = upload_text(NOTEBOOK_ID, title, content)
            if ok:
                print("OK")
                successes += 1
            else:
                print("FALHA")
                failures.append(theme_name)

        # Respiro entre uploads
        if idx < total:
            time.sleep(4)

    print(f"\n{'='*60}")
    print(f"  RESULTADO: {successes}/{total} temas enviados")
    if failures:
        print(f"  Falhas: {', '.join(failures)}")
    print(f"{'='*60}")


if __name__ == "__main__":
    main()
