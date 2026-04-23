"""
SODA Synth v2.0 — Script Unificado de Cristalizacao de Conhecimento
===================================================================
Pipeline tri-modal para sintetizar temas brutos em Manuais Canonicos.

Backends:
  --backend api    -> Google Gemini API (gemini-2.5-flash-lite)
  --backend cli    -> Gemini CLI (gemini-3-flash-preview, pool de quota separado)
  --backend local  -> llama.cpp local via dGPU RTX 2060m (Phi-4-mini-reasoning Q4_K_M)

Uso:
  python soda_synth.py --backend cli
  python soda_synth.py --backend local
  python soda_synth.py --backend api --dry-run
  python soda_synth.py --backend local --themes 21 22 24
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

# ---------------------------------------------------------------------------
# Caminhos
# ---------------------------------------------------------------------------
SCRIPT_DIR = Path(__file__).parent
RAW_DIR    = SCRIPT_DIR / "soda_canon" / "raw"
OUT_DIR    = SCRIPT_DIR / "soda_canon" / "crystalized"
ENV_PATH   = SCRIPT_DIR.parent / ".env"

# Modelo local (Phi-4-mini-reasoning Q4_K_M no LM Studio)
LOCAL_MODEL_PATH = Path(
    r"C:\Users\rosas\.lmstudio\models\lmstudio-community"
    r"\Phi-4-mini-reasoning-GGUF\Phi-4-mini-reasoning-Q4_K_M.gguf"
)
# llama.cpp — sera detectado automaticamente ou configurado aqui
LLAMA_CLI_PATH = None  # Ex: Path(r"C:\llama.cpp\build\bin\Release\llama-cli.exe")

# ---------------------------------------------------------------------------
# Prompt do Arquivista
# ---------------------------------------------------------------------------
SYSTEM_PROMPT = """Voce e o Curador Arquitetural do projeto SODA (Sovereign Operating Data Architecture - Genesis MC).
Sua missao e ler o aglomerado de textos extraidos da web a seguir e sintetiza-lo em um Manual Canonico.
Regras Inegociaveis:
1. ARQUITETURA PURA: A arquitetura SODA usa estritamente Rust (Tokio) no backend e Svelte 5 + Tauri v2 no frontend. O Frontend e uma interface PASSIVA. Toda logica reside no Rust (IPC Zero-Copy).
2. PODA TOXICA: ELIMINE SUMARIAMENTE e ignore paragrafos que exaltem tutoriais de React, Node.js daemons, Electron, VDOM ou arquiteturas Server-Side Rendering (Next.js).
3. HARDWARE AWARE: PRESERVE mencoes a otimizacoes bare-metal, limitacoes da iGPU (gargalos de barramento), e diretivas de execucao AVX2 para a CPU ou llama.cpp mmap para a RTX 2060m.
4. AUDITORIA CRITICA (Furos e Conflitos): Analise ativamente o texto em busca de contradicoes, ideias conflitantes, "furos" ou "buracos" na estrategia do Genesis MC em relacao ao material lido. Se a fonte propoe algo fragil para o SODA, evidencie a falha.
5. CONSOLIDACAO DO "PORQUE": Nao crie um resumo superficial. Consolide o 'PORQUE' tecnico. Una conceitos soltos em uma explicacao coesa.
IMPORTANTE: Retorne APENAS o texto em Markdown puro. Sem saudacoes, sem introducoes como "Aqui esta", sem explicacoes adicionais."""

CONTINUATION_SUFFIX = """

AVISO: Esta e a PARTE {part} deste arquivo. Nao crie uma nova introducao. Apenas adicione os NOVOS axiomas, regras e conceitos tecnicos encontrados nesta parte. Va direto ao ponto."""


# ---------------------------------------------------------------------------
# Utilitarios
# ---------------------------------------------------------------------------
def load_env():
    """Carrega variaveis de ambiente do .env"""
    if ENV_PATH.exists():
        with open(ENV_PATH, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line.startswith("#") or "=" not in line:
                    continue
                key, val = line.split("=", 1)
                val = val.strip("\"'")
                if key in ("GOOGLE_API_KEY", "GEMINI_API_KEY"):
                    os.environ["GEMINI_API_KEY"] = val
                elif key == "GOOGLE_MODEL_FAST":
                    os.environ["GOOGLE_MODEL_FAST"] = val


def discover_pending(specific_themes: list[int] | None = None) -> list[tuple[str, Path, Path]]:
    """Retorna lista de (nome, raw_path, out_path) para temas pendentes."""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    pending = []
    for raw_file in sorted(RAW_DIR.glob("Theme_*.md")):
        theme_name = raw_file.stem  # ex: Theme_21
        out_file = OUT_DIR / f"{theme_name}_canonical.md"

        # Filtrar por temas especificos se fornecidos
        if specific_themes:
            num = int(theme_name.split("_")[1])
            if num not in specific_themes:
                continue

        # Pendente se nao existe ou esta vazio/corrompido (<500 bytes)
        if not out_file.exists() or out_file.stat().st_size < 500:
            pending.append((theme_name, raw_file, out_file))

    return pending


def progress_bar(current: int, total: int, label: str = "", width: int = 40):
    """Barra de progresso ASCII para terminal."""
    pct = current / total if total > 0 else 1
    filled = int(width * pct)
    bar = "#" * filled + "-" * (width - filled)
    sys.stdout.write(f"\r  [{bar}] {current}/{total} {label}")
    sys.stdout.flush()


def validate_output(path: Path) -> bool:
    """Valida que o arquivo canonico gerado e minimamente valido."""
    if not path.exists():
        return False
    size = path.stat().st_size
    if size < 500:
        return False
    # Verificacao rapida: deve conter pelo menos um heading markdown
    with open(path, "r", encoding="utf-8") as f:
        head = f.read(2000)
    return "#" in head


# ---------------------------------------------------------------------------
# Backend: Google Gemini API
# ---------------------------------------------------------------------------
def synth_api(content: str, is_continuation: bool = False, part: int = 1) -> str | None:
    """Sintetiza via API Gemini (gemini-2.5-flash-lite)."""
    from google import genai
    from google.genai import types

    client = genai.Client()
    model = "gemini-2.5-flash-lite"

    instruction = SYSTEM_PROMPT
    if is_continuation:
        instruction += CONTINUATION_SUFFIX.format(part=part)

    retries = 5
    backoff = 120

    for attempt in range(retries):
        try:
            response = client.models.generate_content(
                model=model,
                contents=content,
                config=types.GenerateContentConfig(
                    system_instruction=instruction,
                    temperature=0.2,
                ),
            )
            if response.text and len(response.text.strip()) > 100:
                return response.text.strip()
            print(f"\n  [API] Resposta curta ({len(response.text or '')} chars). Retry {attempt+1}/{retries}")
            time.sleep(15)
        except Exception as e:
            err = str(e).lower()
            if "429" in err or "quota" in err or "resource_exhausted" in err:
                print(f"\n  [API] Rate limit. Aguardando {backoff}s... ({attempt+1}/{retries})")
                time.sleep(backoff)
            else:
                print(f"\n  [API] Erro: {e} ({attempt+1}/{retries})")
                time.sleep(15)
    return None


# ---------------------------------------------------------------------------
# Backend: Gemini CLI
# ---------------------------------------------------------------------------
def synth_cli(content: str, is_continuation: bool = False, part: int = 1) -> str | None:
    """Sintetiza via Gemini CLI (stdin injection, pool gemini-3-flash-preview)."""
    prompt = SYSTEM_PROMPT
    if is_continuation:
        prompt += CONTINUATION_SUFFIX.format(part=part)

    cmd = ["gemini.cmd", "-p", prompt, "--output-format", "json", "--yolo"]

    retries = 5
    backoff = 120

    for attempt in range(retries):
        try:
            result = subprocess.run(
                cmd,
                input=content,
                capture_output=True,
                text=True,
                encoding="utf-8",
                check=False,
                timeout=300,
            )

            if result.returncode != 0:
                err = (result.stderr or "")[:300]
                print(f"\n  [CLI] Exit {result.returncode}. {err[:100]}... ({attempt+1}/{retries})")
                if "quota" in err.lower() or "429" in err:
                    time.sleep(backoff)
                else:
                    time.sleep(15)
                continue

            stdout = result.stdout
            start = stdout.find("{")
            end = stdout.rfind("}") + 1

            if start == -1 or end == 0:
                print(f"\n  [CLI] JSON nao encontrado. ({attempt+1}/{retries})")
                time.sleep(15)
                continue

            data = json.loads(stdout[start:end])

            if "error" in data:
                err_msg = str(data["error"])[:200]
                print(f"\n  [CLI] Erro no JSON: {err_msg} ({attempt+1}/{retries})")
                if "quota" in err_msg.lower() or "429" in err_msg:
                    time.sleep(backoff)
                else:
                    time.sleep(15)
                continue

            resp = data.get("response", "").strip()
            if len(resp) < 100:
                print(f"\n  [CLI] Resposta curta ({len(resp)} chars). ({attempt+1}/{retries})")
                time.sleep(15)
                continue

            return resp

        except subprocess.TimeoutExpired:
            print(f"\n  [CLI] Timeout 5min. ({attempt+1}/{retries})")
            time.sleep(30)
        except json.JSONDecodeError as e:
            print(f"\n  [CLI] JSON invalido: {e}. ({attempt+1}/{retries})")
            time.sleep(15)
        except Exception as e:
            print(f"\n  [CLI] Erro: {e}. ({attempt+1}/{retries})")
            time.sleep(15)

    return None


# ---------------------------------------------------------------------------
# Backend: llama.cpp Local (dGPU RTX 2060m)
# ---------------------------------------------------------------------------
def find_llama_cli() -> Path | None:
    """Detecta o binario llama-cli no sistema."""
    if LLAMA_CLI_PATH and LLAMA_CLI_PATH.exists():
        return LLAMA_CLI_PATH

    # Busca em locais comuns no Windows
    candidates = [
        Path(r"C:\llama.cpp\build\bin\Release\llama-cli.exe"),
        Path(r"C:\llama.cpp\llama-cli.exe"),
        Path.home() / "llama.cpp" / "build" / "bin" / "Release" / "llama-cli.exe",
        Path.home() / "llama.cpp" / "llama-cli.exe",
    ]

    for p in candidates:
        if p.exists():
            return p

    # Tenta via PATH
    try:
        result = subprocess.run(
            ["where", "llama-cli"], capture_output=True, text=True, check=False
        )
        if result.returncode == 0 and result.stdout.strip():
            return Path(result.stdout.strip().splitlines()[0])
    except Exception:
        pass

    return None


def synth_local(content: str, is_continuation: bool = False, part: int = 1) -> str | None:
    """Sintetiza via llama.cpp local com dGPU (RTX 2060m).

    Arquitetura:
    - Modelo: Phi-4-mini-reasoning Q4_K_M (~3.5GB VRAM)
    - Flags: -ngl 99 (offload total GPU), --mmap (obrigatorio tech-stack.md)
    - Env: GGML_CUDA_ENABLE_UNIFIED_MEMORY=1 (spillover RAM quando VRAM esgotar)
    """
    llama_bin = find_llama_cli()
    if not llama_bin:
        print("\n  [LOCAL] ERRO: llama-cli nao encontrado.")
        print("  Instale llama.cpp com CUDA ou configure LLAMA_CLI_PATH no script.")
        print("  Download: https://github.com/ggerganov/llama.cpp/releases")
        print("  Procure: win-cuda-cu12.x-x64.zip")
        return None

    if not LOCAL_MODEL_PATH.exists():
        print(f"\n  [LOCAL] ERRO: Modelo nao encontrado em {LOCAL_MODEL_PATH}")
        return None

    prompt = SYSTEM_PROMPT
    if is_continuation:
        prompt += CONTINUATION_SUFFIX.format(part=part)

    # Monta o prompt completo no formato ChatML
    full_prompt = f"<|system|>\n{prompt}<|end|>\n<|user|>\n{content}<|end|>\n<|assistant|>\n"

    # Arquivo temporario para o prompt (evita limite de tamanho do argumento)
    prompt_file = SCRIPT_DIR / "_temp_prompt.txt"
    try:
        with open(prompt_file, "w", encoding="utf-8") as f:
            f.write(full_prompt)

        env = os.environ.copy()
        env["GGML_CUDA_ENABLE_UNIFIED_MEMORY"] = "1"

        cmd = [
            str(llama_bin),
            "-m", str(LOCAL_MODEL_PATH),
            "-f", str(prompt_file),
            "-ngl", "99",           # Offload total para GPU
            "--mmap",               # Obrigatorio: tech-stack.md
            "-c", "8192",           # Context window
            "-n", "4096",           # Max tokens de saida
            "--temp", "0.2",        # Baixa temperatura para consistencia
            "--top-p", "0.9",
            "--repeat-penalty", "1.1",
            "--no-display-prompt",  # Nao repete o prompt na saida
        ]

        print("\n  [LOCAL] Inferencia via RTX 2060m (Phi-4-mini-reasoning Q4_K_M)...")
        print(f"  [LOCAL] Modelo: {LOCAL_MODEL_PATH.name}")

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=False,
            timeout=600,  # 10 min timeout (modelo local e mais lento)
            env=env,
        )

        if result.returncode != 0:
            err = (result.stderr or "")[:500]
            print(f"\n  [LOCAL] Exit {result.returncode}. Stderr: {err}")
            return None

        output = result.stdout.strip()
        if len(output) < 100:
            print(f"\n  [LOCAL] Saida muito curta ({len(output)} chars).")
            # Mostra stderr para diagnostico
            if result.stderr:
                print(f"  [LOCAL] Stderr: {result.stderr[:300]}")
            return None

        return output

    except subprocess.TimeoutExpired:
        print("\n  [LOCAL] Timeout de 10 minutos excedido.")
        return None
    except Exception as e:
        print(f"\n  [LOCAL] Erro: {e}")
        return None
    finally:
        if prompt_file.exists():
            prompt_file.unlink()


# ---------------------------------------------------------------------------
# Orquestrador Principal
# ---------------------------------------------------------------------------
BACKENDS = {
    "api": synth_api,
    "cli": synth_cli,
    "local": synth_local,
}

CHUNK_SIZE = {
    "api": 800_000,    # API suporta 1M tokens, fatia em 800K chars
    "cli": 800_000,    # CLI idem
    "local": 24_000,   # Local: context window de 8K tokens ~ 24K chars seguros
}

INTER_THEME_DELAY = {
    "api": 30,
    "cli": 45,
    "local": 5,  # Sem rate limit local
}


def process_theme(
    theme_name: str,
    raw_path: Path,
    out_path: Path,
    backend_fn,
    chunk_size: int,
) -> bool:
    """Processa um unico tema com o backend selecionado."""
    with open(raw_path, "r", encoding="utf-8") as f:
        full_content = f.read()

    chunks = [full_content[i : i + chunk_size] for i in range(0, len(full_content), chunk_size)]
    total_chunks = len(chunks)

    # Limpa arquivo de saida
    with open(out_path, "w", encoding="utf-8") as f:
        pass

    for c_idx, chunk in enumerate(chunks):
        if total_chunks > 1:
            progress_bar(c_idx, total_chunks, f"Parte {c_idx+1}/{total_chunks}")

        is_continuation = c_idx > 0
        result = backend_fn(chunk, is_continuation=is_continuation, part=c_idx + 1)

        if not result:
            print(f"\n  FALHA no chunk {c_idx+1}/{total_chunks}")
            # Apaga arquivo parcial para reprocessamento futuro
            if out_path.exists() and out_path.stat().st_size == 0:
                out_path.unlink()
            return False

        with open(out_path, "a", encoding="utf-8") as f:
            if c_idx > 0:
                f.write("\n\n---\n\n")
            f.write(result)

        # Respiro entre chunks
        if c_idx < total_chunks - 1:
            time.sleep(10)

    if total_chunks > 1:
        progress_bar(total_chunks, total_chunks, "Completo")
        print()

    return validate_output(out_path)


def main():
    parser = argparse.ArgumentParser(
        description="SODA Synth v2.0 - Pipeline de Cristalizacao de Conhecimento",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Exemplos:
  python soda_synth.py --backend cli
  python soda_synth.py --backend local
  python soda_synth.py --backend api --dry-run
  python soda_synth.py --backend local --themes 21 22 24
        """,
    )
    parser.add_argument(
        "--backend",
        choices=["api", "cli", "local"],
        default="cli",
        help="Backend de inferencia (default: cli)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Lista temas pendentes sem processar",
    )
    parser.add_argument(
        "--themes",
        nargs="+",
        type=int,
        help="Processar apenas temas especificos (ex: --themes 21 22 24)",
    )

    args = parser.parse_args()

    # Setup
    load_env()
    OUT_DIR.mkdir(parents=True, exist_ok=True)

    backend_name = args.backend.upper()
    backend_fn = BACKENDS[args.backend]
    chunk_size = CHUNK_SIZE[args.backend]
    delay = INTER_THEME_DELAY[args.backend]

    # Descoberta
    pending = discover_pending(args.themes)

    print(f"\n{'='*60}")
    print(f"  SODA Synth v2.0 | Backend: {backend_name}")
    print(f"  Raw: {RAW_DIR}")
    print(f"  Out: {OUT_DIR}")
    if args.backend == "local":
        llama = find_llama_cli()
        print(f"  llama-cli: {llama or 'NAO ENCONTRADO'}")
        print(f"  Modelo: {LOCAL_MODEL_PATH.name}")
        print(f"  VRAM: RTX 2060m 6GB | mmap=ON | CUDA_UNIFIED_MEM=ON")
    print(f"  Chunk size: {chunk_size:,} chars")
    print(f"{'='*60}")

    if not pending:
        print("\n  Nenhum tema pendente. Todos os canonicos estao gerados.")
        print("  Use --themes N para forcar reprocessamento.")
        return

    print(f"\n  Temas pendentes: {len(pending)}")
    for name, raw_p, out_p in pending:
        raw_kb = raw_p.stat().st_size // 1024
        chunks_est = max(1, raw_p.stat().st_size // chunk_size + 1)
        print(f"    - {name} ({raw_kb}KB raw, ~{chunks_est} chunks)")

    if args.dry_run:
        print("\n  [DRY-RUN] Nenhum processamento realizado.")
        return

    # Processamento
    print()
    successes = 0
    failures = []

    for idx, (name, raw_path, out_path) in enumerate(pending):
        raw_kb = raw_path.stat().st_size // 1024
        print(f"  [{idx+1}/{len(pending)}] {name} ({raw_kb}KB)...", end=" ", flush=True)

        ok = process_theme(name, raw_path, out_path, backend_fn, chunk_size)

        if ok:
            out_kb = out_path.stat().st_size // 1024
            print(f"OK ({out_kb}KB gerados)")
            successes += 1
        else:
            print("FALHA")
            failures.append(name)

        # Respiro entre temas
        if idx < len(pending) - 1:
            print(f"  [Anti-ban] Aguardando {delay}s...", flush=True)
            time.sleep(delay)

    # Relatorio
    print(f"\n{'='*60}")
    print(f"  RELATORIO FINAL")
    print(f"  Sucesso: {successes}/{len(pending)}")
    if failures:
        print(f"  Falhas: {', '.join(failures)}")
    print(f"{'='*60}")

    # Verificacao global
    print("\n  Verificacao de integridade:")
    all_canonical = sorted(OUT_DIR.glob("Theme_*_canonical.md"))
    for p in all_canonical:
        size = p.stat().st_size
        status = "OK" if size > 500 else "VAZIO/CORROMPIDO"
        words = size // 6  # estimativa grosseira
        print(f"    {p.name}: {size:,} bytes (~{words:,} palavras) [{status}]")


if __name__ == "__main__":
    main()
