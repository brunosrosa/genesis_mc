"""
Mini-orquestrador: processa apenas os temas PENDENTES via Gemini CLI.
Usa o pool de quota do gemini-3-flash-preview (diferente da API gemini-2.5-flash-lite).
Estratégia: injeção via stdin (sem MCP read_file) para evitar alucinação de agente.
"""

import subprocess
import json
import time
import os

RAW_DIR   = os.path.join(os.path.dirname(__file__), "soda_canon", "raw")
OUT_DIR   = os.path.join(os.path.dirname(__file__), "soda_canon", "crystalized")

# Apenas os temas que ainda não foram cristalizados
PENDING = ["Theme_21.md", "Theme_22.md", "Theme_24.md"]

PROMPT = """Você é o Arquivista Mestre do SODA (Sovereign Operating Data Architecture).
Sua missão: converter o texto bruto abaixo em um Manual Canônico Markdown denso e estruturado.

REGRAS ABSOLUTAS:
1. Retorne APENAS Markdown puro. Nada de explicações, prefácios ou saudações.
2. Preserve toda terminologia técnica (Rust, Tokio, Tauri, SQLite WAL, etc).
3. Organize por seções com ## e ### claros.
4. Destaque conceitos-chave em **negrito**.
5. Condense sem perder precisão técnica.

TEXTO BRUTO PARA CRISTALIZAR:
"""

def call_cli(content: str, retries: int = 5, backoff: int = 120) -> str | None:
    """
    Chama a Gemini CLI injetando o conteúdo via stdin.
    Sem -m flag → CLI auto-seleciona gemini-3-flash-preview (quota pool separada).
    """
    cmd = [
        "gemini.cmd",
        "-p", PROMPT,   # O prompt instrucional vai como argumento -p
        "--output-format", "json",
        "--yolo"
    ]

    for attempt in range(retries):
        try:
            result = subprocess.run(
                cmd,
                input=content,          # Conteúdo bruto via stdin
                capture_output=True,
                text=True,
                encoding="utf-8",
                check=False,
                timeout=300             # 5 min timeout por segurança
            )

            if result.returncode != 0:
                err = result.stderr[:300] if result.stderr else "(sem stderr)"
                print(f"  [Tentativa {attempt+1}/{retries}] Exit {result.returncode}. Stderr: {err}")
                if "quota" in err.lower() or "429" in err.lower() or "QUOTA" in err:
                    print(f"  → Rate Limit. Aguardando {backoff}s...")
                    time.sleep(backoff)
                else:
                    time.sleep(15)
                continue

            # Extrai o JSON da saída (ignora logs do MCP no início)
            stdout = result.stdout
            start = stdout.find('{')
            end   = stdout.rfind('}') + 1

            if start == -1 or end == 0:
                stderr_snippet = result.stderr[:200] if result.stderr else ""
                print(f"  [Tentativa {attempt+1}/{retries}] JSON não encontrado. Stderr: {stderr_snippet}")
                time.sleep(15)
                continue

            data = json.loads(stdout[start:end])

            # Verificar se é erro de quota dentro do JSON
            if "error" in data:
                err_msg = str(data["error"])
                print(f"  [Tentativa {attempt+1}/{retries}] Erro no JSON: {err_msg[:200]}")
                if "quota" in err_msg.lower() or "429" in err_msg:
                    print(f"  → Rate Limit. Aguardando {backoff}s...")
                    time.sleep(backoff)
                else:
                    time.sleep(15)
                continue

            resp = data.get("response", "").strip()
            if len(resp) < 100:
                print(f"  [Tentativa {attempt+1}/{retries}] Resposta muito curta ({len(resp)} chars). Retentando...")
                time.sleep(15)
                continue

            return resp

        except subprocess.TimeoutExpired:
            print(f"  [Tentativa {attempt+1}/{retries}] Timeout de 5min. Retentando...")
            time.sleep(30)
        except json.JSONDecodeError as e:
            print(f"  [Tentativa {attempt+1}/{retries}] JSON inválido: {e}. Retentando...")
            time.sleep(15)
        except Exception as e:
            print(f"  [Tentativa {attempt+1}/{retries}] Erro inesperado: {e}")
            time.sleep(15)

    return None


def main():
    print(f"\n=== Síntese Cirúrgica via CLI (Pool gemini-3-flash-preview) ===")
    print(f"Temas pendentes: {PENDING}\n")

    for theme_file in PENDING:
        raw_path = os.path.join(RAW_DIR, theme_file)
        out_name = theme_file.replace(".md", "_canonical.md")
        out_path = os.path.join(OUT_DIR, out_name)

        if not os.path.exists(raw_path):
            print(f"[SKIP] {theme_file} não encontrado no diretório raw.")
            continue

        print(f"[>>] Processando {theme_file} ({os.path.getsize(raw_path)//1024}KB)...", end=" ", flush=True)

        with open(raw_path, "r", encoding="utf-8") as f:
            content = f.read()

        result = call_cli(content)

        if result:
            with open(out_path, "w", encoding="utf-8") as f:
                f.write(result)
            print(f"OK! ({len(result)//1024}KB gerados)")
        else:
            print(f"\n[FALHA] Não foi possível processar {theme_file}.")

        # Respiro entre temas para não apertar o RPM
        if theme_file != PENDING[-1]:
            print("  [Anti-ban] Aguardando 45s antes do próximo...", flush=True)
            time.sleep(45)

    print("\n=== Verificação Final ===")
    for theme_file in PENDING:
        out_name = theme_file.replace(".md", "_canonical.md")
        out_path = os.path.join(OUT_DIR, out_name)
        if os.path.exists(out_path):
            size = os.path.getsize(out_path)
            status = "OK" if size > 500 else "VAZIO"
            print(f"  {out_name}: {size} bytes {status}")
        else:
            print(f"  {out_name}: AUSENTE")


if __name__ == "__main__":
    main()
