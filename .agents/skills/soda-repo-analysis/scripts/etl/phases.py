"""
phases.py — As 3 Fases do Pipeline Cognitivo ETL.

T-04: phase1_kimi()     — Triagem e Contexto (Kimi K2 via OpenRouter)
T-05: phase2_swarm()    — Map-Reduce Socrático (3 Lentes SODA em asyncio.gather)
T-06: phase3_validate() — Síntese Pydantic AI + Regras de Classificação Terminal

Regra FinOps: Zero GPU local. Todas as chamadas passam pelo OpenRouter.
A CPU orquestra apenas asyncio.gather e o parsing JSON. VRAM intacta.
"""
from __future__ import annotations

import asyncio
import json
import logging
import os
import re
import sqlite3
from datetime import datetime
from typing import Any

import httpx

from etl.db import log_error
from etl.models import (
    LenteOutput,
    RepoContext,
    RepoHeuristic,
    SwarmResult,
    classificar,
)

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Configuração OpenRouter
# ---------------------------------------------------------------------------
_OR_BASE_URL = os.getenv("OPENAI_BASE_URL", "https://openrouter.ai/api/v1")
_OR_KEY_FAST  = os.getenv("OPENROUTER_API_FAST", "")
_OR_KEY_HEAVY = os.getenv("OPENROUTER_API_HEAVY", "")

_MODEL_KIMI      = os.getenv("OPENROUTER_FAST_MODEL",            "moonshotai/kimi-k2.5")
_MODEL_FORMATTER = os.getenv("OPENROUTER_FORMATTER_MODEL",       "deepseek/deepseek-chat")
_MODEL_LENS_UX   = os.getenv("OPENROUTER_HEAVY_MODEL_LENS_UX",  "anthropic/claude-opus-4.7")
_MODEL_LENS_ARQ  = os.getenv("OPENROUTER_HEAVY_MODEL_LENS_ARQ", "deepseek/deepseek-v4-pro")
_MODEL_LENS_OPS  = os.getenv("OPENROUTER_HEAVY_MODEL_LENS_OPS", "z-ai/glm-5.1")

_HEADERS_FAST  = {"Authorization": f"Bearer {_OR_KEY_FAST}",  "Content-Type": "application/json"}
_HEADERS_HEAVY = {"Authorization": f"Bearer {_OR_KEY_HEAVY}", "Content-Type": "application/json"}

# Timeouts (segundos)
_TIMEOUT_KIMI  = 30.0
_TIMEOUT_LENTE = 180.0
_TIMEOUT_SYNTH = 60.0

# ---------------------------------------------------------------------------
# Prompts
# ---------------------------------------------------------------------------
_KIMI_SYSTEM = (
    "You are a senior software architect. Analyze the given GitHub repository URL and README content. "
    "TRADUZA TODAS AS DESCRIÇÕES ESTRITAMENTE PARA O PORTUGUÊS (PT-BR). Resuma o README focando no propósito real. "
    "Return a JSON object with these exact keys: "
    "primary_language (str), domain_hint (str: one of web-framework/cli-tool/ml-lib/"
    "embedded/database/unknown), summary (str, max 300 chars), "
    "has_rust_components (bool), has_wasm_targets (bool), "
    "estimated_complexity (str: LOW/MED/HIGH). Return ONLY valid JSON."
)

_LENTE_A_SYSTEM = (
    "Você é a Lente UX/Produto. Analise o atrito humano, a utilidade e a viabilidade da interface "
    "para integração em um ecossistema bare-metal Rust (SODA). "
    "CRITICAL RULE: PROIBIDO ler ou referenciar dados antigos. Evite Anchoring Bias. "
    "DIRETRIZ DE EQUILÍBRIO: O SODA odeia runtimes tóxicos (Node.js/Python), mas AMA abstrações e heurísticas geniais. "
    "Se a linguagem original for tóxica, puna severamente o 'bare_metal_fit' e a 'operability_level'. "
    "NO ENTANTO, se a Visão de Produto (UX), a Lógica Matemática ou o Paradigma forem excelentes, exalte-os! "
    "DIRETRIZ DE IDIOMA: Toda a sua resposta DEVE ser rigorosamente em Português do Brasil. "
    "Return JSON with keys: raw_analysis (str, max 800 chars), score_parcial (float 0-10), "
    "flags (list[str]). ONLY valid JSON."
)

_LENTE_B_SYSTEM = (
    "Você é a Lente Bare-Metal/Arquitetura. Analise o fit para 6GB VRAM, o uso de Rust/C, "
    "dependências tóxicas (Node.js/Python) e a viabilidade de canibalização da lógica O(1) "
    "para integração em um ecossistema Rust (SODA). "
    "CRITICAL RULE: PROIBIDO ler ou referenciar dados antigos. Evite Anchoring Bias. "
    "DIRETRIZ DE EQUILÍBRIO: O SODA odeia runtimes tóxicos, mas AMA abstrações geniais. "
    "DIRETRIZ DE IDIOMA: Toda a sua resposta DEVE ser rigorosamente em Português do Brasil. "
    "Return JSON with keys: raw_analysis (str, max 800 chars), score_parcial (float 0-10), "
    "flags (list[str]). ONLY valid JSON."
)

_LENTE_C_SYSTEM = (
    "Você é a Lente Operacional. Avalie a sustentação 24/7, risco de entropia e entropia de manutenção "
    "em um contexto bare-metal embarcado (SODA). "
    "CRITICAL RULE: PROIBIDO ler ou referenciar dados antigos. Evite Anchoring Bias. "
    "DIRETRIZ DE IDIOMA: Toda a sua resposta DEVE ser rigorosamente em Português do Brasil. "
    "Return JSON with keys: raw_analysis (str, max 800 chars), score_parcial (float 0-10), "
    "flags (list[str]). ONLY valid JSON."
)

_SYNTHESIS_SYSTEM = (
    "You are a CTO synthesizing three specialist analyses. "
    "Baseado estritamente nas 3 lentes, classifique a ferramenta. NÃO invente valores fora das opções permitidas. "
    "Aplique rigidez. Responda 100% em Português.\n"
    "LEI DOS SCORES (10 = Ideal para SODA, 0 = Tóxico):\n"
    "1. O `score_final` NÃO É UMA MÉDIA ARITMÉTICA. Ele mede a 'Gravidade da Ideia' (Poder de Canibalização). Se a sacada do produto for genial, o score_final DEVE ser alto (ex: 9.5), mesmo que a stack original seja tóxica e o `score_bare_metal_fit` seja 0.\n"
    "2. O `score_operability` mede o atrito do 'Day After'. Se a Lente C apontar manutenção infernal ou dependência de nuvem, derrube esta nota sumariamente.\n"
    "3. Não suavize notas de VRAM/Hardware. Se exige Node.js, Electron ou mais de 6GB de VRAM, o `score_bare_metal_fit` deve ser implacavelmente destruído.\n"
    "Return a JSON object containing exactly these string fields: "
    "executive_verdict, entropy_risk, design_misuse_risk, intrinsic_ethics_risk, "
    "horizonte_extracao, justificativa_decisao, categoria_nuance_tecnica, "
    "stack_base, tipo_integracao, integracao_papel_exato, must_components, "
    "proposta_original_resumo, ouro_a_extrair, deep_pattern, acao_de_canibalizacao, "
    "transplantable_core, logic_math_heuristic, risco_principal, risco_linha_vermelha, "
    "observacoes, real_structural_problem, bare_metal_fit, discipline_dependency, "
    "extractability_level, operability_level, where_ai_should_not_enter, do_not_absorb, visao_do_enxame. "
    "ONLY valid JSON."
)

# ---------------------------------------------------------------------------
# Helper HTTP
# ---------------------------------------------------------------------------

async def _openrouter_chat(
    model: str,
    system: str,
    user_msg: str,
    headers: dict[str, str],
    timeout: float,
) -> dict[str, Any]:
    """Faz uma chamada chat ao OpenRouter e retorna o JSON parseado da resposta."""
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user",   "content": user_msg},
        ],
        "response_format": {"type": "json_object"},
        "temperature": 0.2,
        "max_tokens": 4000,
    }
    async with httpx.AsyncClient(timeout=timeout) as client:
        resp = await client.post(
            f"{_OR_BASE_URL}/chat/completions",
            headers=headers,
            json=payload,
        )
        resp.raise_for_status()
    data = resp.json()
    content = data["choices"][0]["message"]["content"]
    if content is None:
        raise ValueError(
            f"Modelo '{model}' retornou content=None. "
            "Possível: streaming ativo, quota excedida ou resposta vazia."
        )
    return json.loads(content)


async def _run_ephemeral_cli(cmd_str: str, timeout: float = 30.0, adaptive_ceiling: float = 0.0) -> str:
    """
    Executa um sidecar CLI de forma efêmera (Higiene de RAM).
    Se adaptive_ceiling > 0, faz retries incrementando timeout em +15s até atingir o teto.
    """
    current_timeout = timeout
    while True:
        proc = await asyncio.create_subprocess_shell(
            cmd_str,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=current_timeout)
            if proc.returncode != 0:
                logger.warning(f"[SIDECAR] Exit {proc.returncode} em '{cmd_str}': {stderr.decode('utf-8', errors='ignore')[:200]}")
                return ""
            return stdout.decode('utf-8', errors='ignore')
        except asyncio.TimeoutError:
            logger.warning(f"[SIDECAR] Timeout em '{cmd_str}' com {current_timeout}s")
            if proc.returncode is None:
                try:
                    proc.kill()
                except Exception:
                    pass
            if adaptive_ceiling > 0:
                current_timeout += 15.0
                if current_timeout > adaptive_ceiling:
                    logger.error(f"[SIDECAR] Hard Ceiling atingido ({adaptive_ceiling}s) para '{cmd_str}'.")
                    raise RuntimeError(f"Circuit Breaker: Timeout Hard Ceiling {adaptive_ceiling}s atingido em {cmd_str}")
                logger.info(f"[SIDECAR] Retry adaptativo: novo timeout = {current_timeout}s")
                continue
            return ""
        except Exception as e:
            logger.error(f"[SIDECAR] Erro em '{cmd_str}': {e}")
            return ""
        finally:
            if proc.returncode is None:
                try:
                    proc.kill()  # SIGKILL atômico: impede zumbis na RAM/VRAM
                except Exception:
                    pass
        break

async def _run_mcp_tool(cmd_str: str, tool_name: str, tool_args: dict, timeout: float = 60.0, adaptive_ceiling: float = 0.0) -> str:
    """
    Executa um sidecar MCP via stdio (JSON-RPC 2.0).
    Injeta initialize, aguarda, injeta tools/call, aguarda resposta e dá SIGKILL atômico.
    Se adaptive_ceiling > 0, tenta novamente incrementando o timeout.
    """
    current_timeout = timeout
    while True:
        proc = await asyncio.create_subprocess_shell(
            cmd_str,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE
        )
        try:
            init_req = {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "soda-etl", "version": "1.0.0"}}}
            proc.stdin.write((json.dumps(init_req) + "\n").encode("utf-8"))
            await proc.stdin.drain()
            
            while True:
                line = await asyncio.wait_for(proc.stdout.readline(), timeout=15.0)
                if not line: break
                try:
                    resp = json.loads(line.decode("utf-8").strip())
                    if resp.get("id") == 1: break
                except Exception: pass
                    
            init_notif = {"jsonrpc": "2.0", "method": "notifications/initialized"}
            call_req = {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": tool_name, "arguments": tool_args}}
            proc.stdin.write((json.dumps(init_notif) + "\n").encode("utf-8"))
            proc.stdin.write((json.dumps(call_req) + "\n").encode("utf-8"))
            await proc.stdin.drain()
            
            result_text = ""
            while True:
                line = await asyncio.wait_for(proc.stdout.readline(), timeout=current_timeout)
                if not line: break
                try:
                    resp = json.loads(line.decode("utf-8").strip())
                    if resp.get("id") == 2:
                        if "error" in resp:
                            result_text = f"Error: {resp['error']}"
                        else:
                            content = resp.get("result", {}).get("content", [])
                            if content and isinstance(content, list):
                                result_text = content[0].get("text", "")
                            else:
                                result_text = json.dumps(resp.get("result"))
                        break
                except Exception: pass
            return result_text
        except asyncio.TimeoutError:
            logger.warning(f"[SIDECAR-MCP] Timeout em '{tool_name}' com {current_timeout}s")
            if proc.returncode is None:
                try:
                    proc.kill()
                except Exception:
                    pass
            if adaptive_ceiling > 0:
                current_timeout += 15.0
                if current_timeout > adaptive_ceiling:
                    logger.error(f"[SIDECAR-MCP] Hard Ceiling atingido ({adaptive_ceiling}s) para '{tool_name}'.")
                    raise RuntimeError(f"Circuit Breaker: Timeout Hard Ceiling {adaptive_ceiling}s atingido em {tool_name}")
                logger.info(f"[SIDECAR-MCP] Retry adaptativo: novo timeout = {current_timeout}s")
                continue
            return ""
        except Exception as e:
            logger.error(f"[SIDECAR-MCP] Erro em '{tool_name}': {e}")
            return ""
        finally:
            if proc.returncode is None:
                try:
                    proc.kill()
                except Exception:
                    pass
        break


# ---------------------------------------------------------------------------
# T-04: Fase 1 — Kimi K2 (Triagem e Extração de Contexto)
# ---------------------------------------------------------------------------

async def phase1_kimi(
    repo_url: str,
    conn: sqlite3.Connection,
    run_id: str,
    repo_id: str,
) -> RepoContext:
    """
    SODA Fase 1: Extração 'Raw' local-first + Kimi K2.
    Usa GITHUB_PAT da RAM efêmera para extrair o README físico burlando 403.
    """
    import os, re, httpx
    
    # 1. Extrair owner/repo da URL
    match = re.search(r"github\.com/([^/]+/[^/]+)", repo_url)
    repo_path = match.group(1).replace(".git", "") if match else ""
    
    # 2. Protocolo Anti-404 e Extração HTTPX
    github_pat = os.environ.get("GITHUB_PAT")
    headers = {"Authorization": f"Bearer {github_pat}"} if github_pat else {}
    
    metadata_text = ""
    readme_text = ""
    
    if repo_path:
        try:
            async with httpx.AsyncClient(timeout=15.0) as client:
                # Anti-404 (Bate na raiz da API)
                resp = await client.get(f"https://api.github.com/repos/{repo_path}", headers=headers)
                if resp.status_code in (404, 403, 401):
                    logger.error(f"[FASE-1][{repo_id}] 404/403/401 API Github. Short-Circuit ativado.")
                    raise RuntimeError(f"Falso 404/Dead Link: {repo_path} inacessível (Status {resp.status_code}).")
                
                resp.raise_for_status()
                data = resp.json()
                metadata_text = (
                    f"Stars: {data.get('stargazers_count')} | Forks: {data.get('forks_count')} | "
                    f"Lang: {data.get('language')} | Topics: {data.get('topics')}\n"
                    f"Desc: {data.get('description')}\n"
                )[:500]

                # README
                headers["Accept"] = "application/vnd.github.v3.raw"
                rm_resp = await client.get(f"https://api.github.com/repos/{repo_path}/readme", headers=headers)
                if rm_resp.status_code == 200:
                    readme_text = rm_resp.text[:8000] # Teto FinOps: 8k
        except RuntimeError as e:
            raise e # Short-circuit bolha
        except Exception as e:
            logger.warning(f"[FASE-1][{repo_id}] Falha HTTPX: {e}")

    # 3. Orquestração O(1) via Sidecars (JCodeMunch & Webcrawl)
    ast_text = ""
    wiki_text = ""
    if repo_path:
        logger.info(f"[FASE-1][{repo_id}] Invocando subprocessos efêmeros AST/Webcrawl...")
        ast_raw = await _run_mcp_tool(
            cmd_str="uvx --from jcodemunch-mcp jcodemunch-mcp",
            tool_name="get_file_outline",
            tool_args={"repo": repo_path},
            timeout=45.0
        )
        ast_text = ast_raw[:6500] # Teto FinOps: 6.5k
        
        try:
            wiki_raw = await _run_mcp_tool(
                cmd_str="uvx --from webcrawl-mcp webcrawl-mcp",
                tool_name="webcrawl_scrape",
                tool_args={"url": f"https://github.com/{repo_path}/wiki"},
                timeout=30.0,
                adaptive_ceiling=120.0
            )
            wiki_text = wiki_raw[:2000]
        except RuntimeError as e:
            # Circuit breaker atingido
            logger.error(f"[FASE-1][{repo_id}] {e}")
            wiki_text = "[Falha: Timeout Adaptativo atingiu Hard Ceiling]"

    # Concatenar e aplicar Teto Global de FinOps (15k chars)
    full_context = (
        f"--- METADATA ---\n{metadata_text}\n"
        f"--- README ---\n{readme_text}\n"
        f"--- AST OUTLINE ---\n{ast_text}\n"
        f"--- WIKI ---\n{wiki_text}"
    )[:15000]

    os.makedirs(".logs/raw_dumps", exist_ok=True)
    with open(f".logs/raw_dumps/{repo_id.replace('/', '_')}_phase1_raw.txt", "w", encoding="utf-8") as f:
        f.write(full_context)

    # 4. Injeção de Contexto Ancorado (Verdade Oficial)
    user_msg = (
        f"Analyze this GitHub repository and return the JSON.\n"
        f"URL: {repo_url}\n\n"
        f"--- RAW REPOSITORY CONTENT (TRIPLE EXTRACTION) ---\n"
        f"{full_context}\n"
        f"--------------------------"
    )
    
    try:
        raw = await _openrouter_chat(
            model=_MODEL_KIMI,
            system=_KIMI_SYSTEM,
            user_msg=user_msg,
            headers=_HEADERS_FAST,
            timeout=_TIMEOUT_KIMI,
        )
        ctx = RepoContext.model_validate(raw)
        logger.info("[FASE-1][%s] Kimi OK — lang=%s domain=%s", repo_id, ctx.primary_language, ctx.domain_hint)
        return ctx
    except Exception as exc:
        logger.warning("[FASE-1][%s] Kimi falhou: %s — usando fallback", repo_id, exc)
        log_error(conn, run_id, repo_id, fase=1, exc=exc)
        return RepoContext(
            primary_language="unknown",
            domain_hint="unknown",
            summary="[Fase 1 falhou — contexto parcial]",
            estimated_complexity="MED",
        )


# ---------------------------------------------------------------------------
# T-05: Fase 2 — Map-Reduce Socrático (3 Lentes SODA em paralelo)
# ---------------------------------------------------------------------------

async def _call_lente(
    model: str,
    system: str,
    user_msg: str,
    headers: dict[str, str],
    lente_name: str,
    repo_id: str,
) -> LenteOutput:
    """Corotina individual de uma Lente. Falha isolada — não propaga para o gather."""
    raw = await _openrouter_chat(
        model=model,
        system=system,
        user_msg=user_msg,
        headers=headers,
        timeout=_TIMEOUT_LENTE,
    )
    output = LenteOutput.model_validate(raw)
    logger.info("[FASE-2][%s] %s OK — score=%.1f flags=%s", repo_id, lente_name, output.score_parcial, output.flags)
    return output


async def phase2_swarm(
    ctx: RepoContext,
    repo_url: str,
    conn: sqlite3.Connection,
    run_id: str,
    repo_id: str,
) -> SwarmResult:
    """
    Despacha as 3 Lentes SODA simultaneamente via asyncio.gather(return_exceptions=True).
    Falha isolada de uma Lente registra em etl_errors(fase=2) e preenche lente=None.
    Nunca aborta o lote — SwarmResult.lentes_disponiveis indica a qualidade do dado.
    """
    # 0. Aterramento Semântico via NotebookLM (Sidecar Efêmero)
    logger.info(f"[FASE-2][{repo_id}] Invocando NotebookLM para Aterramento SODA...")
    notebooklm_context = ""
    try:
        nl_query = f"Heurísticas SODA e diretrizes bare-metal para repo tipo '{ctx.domain_hint}' e linguagem '{ctx.primary_language}'."
        
        # Comunicação atômica via JSON-RPC sobre stdio (NotebookLM MCP)
        nl_cmd = "uvx --quiet --from notebooklm-mcp-cli notebooklm-mcp"
        nl_args = {
            "notebook_id": "0737996f-cf30-4050-a9a8-e18a16899937",
            "query": nl_query
        }
        nl_raw = await _run_mcp_tool(nl_cmd, "notebooklm_notebook_query", nl_args, timeout=60.0)
        
        if not nl_raw or "error" in nl_raw.lower() or "auth" in nl_raw.lower():
            raise ValueError(f"NotebookLM falhou ou não autorizado. Retorno: {nl_raw[:100]}")
        notebooklm_context = nl_raw[:3000]
        
        os.makedirs(".logs/raw_dumps", exist_ok=True)
        with open(f".logs/raw_dumps/{repo_id.replace('/', '_')}_notebooklm.txt", "w", encoding="utf-8") as f:
            f.write(notebooklm_context)
    except Exception as e:
        logger.error(f"[FASE-2][{repo_id}] Falha Crítica de Aterramento Semântico (Auth/Cookie): {e}")
        # CRITICAL RULE: Crashar o lote, não seguir cego.
        raise RuntimeError(f"FALHA SODA CANON (NOTEBOOKLM): Incapaz de consultar as regras do projeto. {e}")

    readme_hint = (
        f"Repository: {repo_url}\n"
        f"Language: {ctx.primary_language} | Domain: {ctx.domain_hint}\n"
        f"Complexity: {ctx.estimated_complexity} | "
        f"Rust: {ctx.has_rust_components} | Wasm: {ctx.has_wasm_targets}\n"
        f"Summary: {ctx.summary}\n\n"
        f"--- SODA CANON (NOTEBOOKLM GROUNDING) ---\n"
        f"{notebooklm_context}\n"
        f"-----------------------------------------"
    )

    results = await asyncio.gather(
        _call_lente(_MODEL_LENS_UX,  _LENTE_A_SYSTEM, readme_hint, _HEADERS_HEAVY, "Lente-A-UX",  repo_id),
        _call_lente(_MODEL_LENS_ARQ, _LENTE_B_SYSTEM, readme_hint, _HEADERS_HEAVY, "Lente-B-ARQ", repo_id),
        _call_lente(_MODEL_LENS_OPS, _LENTE_C_SYSTEM, readme_hint, _HEADERS_HEAVY, "Lente-C-OPS", repo_id),
    )

    lente_a, lente_b, lente_c = results
    lentes_ok = 3

    os.makedirs(".logs/raw_dumps", exist_ok=True)
    with open(f".logs/raw_dumps/{repo_id.replace('/', '_')}_lentes.txt", "w", encoding="utf-8") as f:
        f.write(f"--- LENTE A ---\n{lente_a.model_dump_json(indent=2) if lente_a else 'FAIL'}\n\n")
        f.write(f"--- LENTE B ---\n{lente_b.model_dump_json(indent=2) if lente_b else 'FAIL'}\n\n")
        f.write(f"--- LENTE C ---\n{lente_c.model_dump_json(indent=2) if lente_c else 'FAIL'}\n\n")

    logger.info("[FASE-2][%s] Todas as 3 lentes concluídas com sucesso (Fail-Fast)", repo_id)

    logger.info("[FASE-2][%s] %d/3 lentes disponíveis", repo_id, lentes_ok)
    return SwarmResult(
        lente_a=lente_a,
        lente_b=lente_b,
        lente_c=lente_c,
        lentes_disponiveis=lentes_ok,
    )


# ---------------------------------------------------------------------------
# T-06: Fase 3 — Síntese Pydantic AI + Classificação Terminal
# ---------------------------------------------------------------------------

def _weighted_score(swarm: SwarmResult) -> tuple[float, float, float, float, float, float, float]:
    """
    Calcula os 6 scores granulares e o score_total a partir das Lentes disponíveis.
    Na ausência de lentes específicas, usa médias das disponíveis como proxy.
    Retorna: (total, arq, rust, bare, wasm, lat, manut)
    """
    scores_parciais = [
        l.score_parcial
        for l in [swarm.lente_a, swarm.lente_b, swarm.lente_c]
        if l is not None
    ]
    if not scores_parciais:
        return (0.0,) * 7

    # Pesos: Lente B (Arquitetura) tem maior influência no score_total
    # Lente A → UX (produto): influencia rust_potential e manutencao
    # Lente B → Arq: influencia score_arquitetura, bare_metal, wasm_compat
    # Lente C → Ops: influencia latencia, manutencao
    sa = swarm.lente_a.score_parcial if swarm.lente_a else sum(scores_parciais) / len(scores_parciais)
    sb = swarm.lente_b.score_parcial if swarm.lente_b else sum(scores_parciais) / len(scores_parciais)
    sc = swarm.lente_c.score_parcial if swarm.lente_c else sum(scores_parciais) / len(scores_parciais)

    score_arq          = round(sb, 2)
    score_rust         = round((sb * 0.6 + sa * 0.4), 2)
    score_bare_metal   = round((sb * 0.7 + sc * 0.3), 2)
    score_wasm         = round((sb * 0.6 + sc * 0.4), 2)
    score_lat          = round((sc * 0.6 + sb * 0.4), 2)
    score_manut        = round((sc * 0.5 + sa * 0.3 + sb * 0.2), 2)

    # Score total: média ponderada (B=40%, A=30%, C=30%)
    total = round((sb * 0.40 + sa * 0.30 + sc * 0.30), 2)
    total = max(0.0, min(10.0, total))

    return total, score_arq, score_rust, score_bare_metal, score_wasm, score_lat, score_manut


async def phase3_validate(
    swarm: SwarmResult,
    ctx: RepoContext,
    repo_url: str,
    repo_id: str,
    lote_id: str,
    nome_projeto: str,
    conn: sqlite3.Connection,
    run_id: str,
) -> RepoHeuristic:
    """
    Sintetiza SwarmResult + RepoContext → RepoHeuristic canônico.
    Gera executive_verdict via OpenRouter (deepseek-chat — custo baixo).
    Calcula score_total e aplica regras de classificação terminal.
    """
    (
        score_total, score_arq, score_rust,
        score_bare, score_wasm, score_lat, score_manut,
    ) = _weighted_score(swarm)

    classificacao = classificar(score_total)

    # Síntese executiva via OpenRouter
    synthesis_input = (
        f"Repository: {repo_url}\n"
        f"Domain: {ctx.domain_hint} | Language: {ctx.primary_language}\n"
        f"Score total: {score_total:.1f}/10 → {classificacao}\n"
        f"Lente A (UX): {swarm.lente_a.raw_analysis[:200] if swarm.lente_a else 'N/A'}\n"
        f"Lente B (Arq): {swarm.lente_b.raw_analysis[:200] if swarm.lente_b else 'N/A'}\n"
        f"Lente C (Ops): {swarm.lente_c.raw_analysis[:200] if swarm.lente_c else 'N/A'}"
    )

    executive_verdict = "[Síntese indisponível]"
    raw_synth = {}
    try:
        raw_synth = await _openrouter_chat(
            model=_MODEL_FORMATTER,
            system=_SYNTHESIS_SYSTEM,
            user_msg=synthesis_input,
            headers=_HEADERS_FAST,
            timeout=_TIMEOUT_SYNTH,
        )
        executive_verdict = raw_synth.get("executive_verdict", executive_verdict)[:400]
    except Exception as exc:
        logger.warning("[FASE-3][%s] Síntese falhou: %s", repo_id, exc)
        log_error(conn, run_id, repo_id, fase=3, exc=exc)

    # Extrai categoria arquitetural dos flags da Lente B
    categoria = _infer_categoria(swarm, ctx)
    
    # Prepara o construtor com defaults de segurança caso o LLM falhe
    def _get(key: str, default: str) -> str:
        return str(raw_synth.get(key, default))

    # Constrói justificativa a partir dos flags disponíveis
    all_flags: list[str] = []
    for lente in [swarm.lente_a, swarm.lente_b, swarm.lente_c]:
        if lente:
            all_flags.extend(lente.flags)
    justificativa = f"Score={score_total:.1f} | Flags: {', '.join(all_flags[:10])}"[:600]

    return RepoHeuristic(
        project_name=nome_projeto,
        declared_description=ctx.summary[:300],
        repo_url=repo_url,
        score_final=score_total,
        score_fit_geral_soda=(score_arq + score_rust + score_bare) / 3,
        score_philosophical_fit=score_rust,
        score_bare_metal_fit=score_bare,
        score_architectural_extractability=score_arq,
        score_operability=score_lat,
        score_creep_risk=score_manut,
        entropy_risk=_get("entropy_risk", "HIGH") if _get("entropy_risk", "HIGH") in ["LOW", "MEDIUM", "HIGH", "CRITICAL"] else "HIGH",
        design_misuse_risk=_get("design_misuse_risk", "Pendente"),
        intrinsic_ethics_risk=_get("intrinsic_ethics_risk", "Pendente"),
        horizonte_extracao=_get("horizonte_extracao", "CURTO_MEDIO_PRAZO") if _get("horizonte_extracao", "CURTO_MEDIO_PRAZO") in ["IMEDIATO", "CURTO_PRAZO", "CURTO_MEDIO_PRAZO", "MEDIO_PRAZO", "LONGO_PRAZO", "REFERENCIAL_TEORICO"] else "CURTO_MEDIO_PRAZO",
        justificativa_decisao=_get("justificativa_decisao", justificativa)[:400],
        categoria_arquitetural=_get("categoria_arquitetural", "Tooling") if _get("categoria_arquitetural", "Tooling") in ["Canvas", "Interface", "Memória", "Roteamento", "Orquestração", "Segurança", "Infraestrutura", "Tooling"] else "Tooling",
        categoria_nuance_tecnica=_get("categoria_nuance_tecnica", categoria),
        classificacao_terminal=classificacao,
        stack_base=_get("stack_base", ctx.primary_language),
        tipo_integracao=_get("tipo_integracao", "Sidecar Efêmero") if _get("tipo_integracao", "Sidecar Efêmero") in ["Biblioteca / Crate Nativa", "Sidecar Efêmero", "Daemon / Background Service", "App Nativo / CLI Independente", "Middleware / Proxy"] else "Sidecar Efêmero",
        integracao_papel_exato=_get("integracao_papel_exato", "Pendente"),
        must_components=_get("must_components", "Pendente"),
        proposta_original_resumo=_get("proposta_original_resumo", ctx.summary[:200]),
        lente_a_sentido_ux=swarm.lente_a.raw_analysis[:600] if swarm.lente_a else "N/A",
        lente_b_estrutura_arq=swarm.lente_b.raw_analysis[:600] if swarm.lente_b else "N/A",
        lente_c_realidade_ops=swarm.lente_c.raw_analysis[:600] if swarm.lente_c else "N/A",
        visao_do_enxame=_get("visao_do_enxame", "Pendente")[:600],
        executive_verdict=executive_verdict,
        ouro_a_extrair=_get("ouro_a_extrair", "Pendente"),
        deep_pattern=_get("deep_pattern", "Pendente"),
        acao_de_canibalizacao=_get("acao_de_canibalizacao", "No Absorption") if _get("acao_de_canibalizacao", "No Absorption") in ["Data Model / Schema", "Prompt / Heuristic Seed", "Protocol / Standard", "Concept", "UX Pattern", "Canvas Refinement", "New Canvas", "Cognitive Layer", "Infra Capability", "Technical Runtime", "Sandbox", "Plugin", "External Contract", "No Absorption"] else "No Absorption",
        transplantable_core=_get("transplantable_core", "Pendente"),
        logic_math_heuristic=_get("logic_math_heuristic", "Pendente"),
        risco_principal=_get("risco_principal", "Risco não avaliado"),
        risco_linha_vermelha=_get("risco_linha_vermelha", "Pendente"),
        observacoes=_get("observacoes", "Nenhuma"),
        real_structural_problem=_get("real_structural_problem", "Pendente"),
        bare_metal_fit=_get("bare_metal_fit", "MEDIUM") if _get("bare_metal_fit", "MEDIUM") in ["LOW", "MEDIUM", "HIGH", "EXCELLENT"] else "MEDIUM",
        discipline_dependency=_get("discipline_dependency", "Média (Conformidade Básica)") if _get("discipline_dependency", "Média (Conformidade Básica)") in ["Nenhuma (Automação Invisível)", "Baixa (Tolerante a Erros)", "Média (Conformidade Básica)", "Alta (Exige Mudança de Hábito)", "Crítica (Quebra sem Disciplina)"] else "Média (Conformidade Básica)",
        extractability_level=_get("extractability_level", "MEDIUM") if _get("extractability_level", "MEDIUM") in ["LOW", "MEDIUM", "HIGH", "EXCELLENT"] else "MEDIUM",
        operability_level=_get("operability_level", "MEDIUM") if _get("operability_level", "MEDIUM") in ["LOW", "MEDIUM", "HIGH", "EXCELLENT"] else "MEDIUM",
        where_ai_should_not_enter=_get("where_ai_should_not_enter", "Pendente"),
        do_not_absorb=_get("do_not_absorb", "Pendente"),
        data_ultima_analise=datetime.now().isoformat(timespec='seconds'),
        analise_origem="SODA ETL V3 Auto",
        lote_id=lote_id,
    )


# ---------------------------------------------------------------------------
# Helpers Internos
# ---------------------------------------------------------------------------

_CATEGORIA_FLAG_MAP: dict[str, str] = {
    "has_tokio": "Technical Infrastructure",
    "ffi_friendly": "Technical Infrastructure",
    "heavy_deps": "Tooling / Support",
    "viral_license": "Tooling / Support",
    "no_tests": "Tooling / Support",
    "has_wasm": "Capability Layer",
}

def _infer_categoria(swarm: SwarmResult, ctx: RepoContext) -> str:
    """Infere categoria arquitetural a partir dos flags da Lente B e do domain_hint."""
    if swarm.lente_b:
        for flag in swarm.lente_b.flags:
            if flag in _CATEGORIA_FLAG_MAP:
                return _CATEGORIA_FLAG_MAP[flag]
    # Fallback: usa domain_hint
    return ctx.domain_hint if ctx.domain_hint != "unknown" else "Tooling / Support"

def create_rejected_heuristic(repo_id: str, repo_url: str, nome_projeto: str, lote_id: str) -> RepoHeuristic:
    """Gera um RepoHeuristic estrito de rejeição (Short-Circuit) para repositórios vazios/inacessíveis."""
    return RepoHeuristic(
        project_name=nome_projeto,
        declared_description="Repositório inacessível ou vazio.",
        repo_url=repo_url,
        score_final=0.0,
        score_fit_geral_soda=0.0,
        score_philosophical_fit=0.0,
        score_bare_metal_fit=0.0,
        score_architectural_extractability=0.0,
        score_operability=0.0,
        score_creep_risk=0.0,
        entropy_risk="CRITICAL",
        design_misuse_risk="N/A",
        intrinsic_ethics_risk="N/A",
        horizonte_extracao="REFERENCIAL_TEORICO",
        justificativa_decisao="Repositório inacessível ou morto. Abortado via Short-Circuit na Fase 1.",
        categoria_arquitetural="Tooling",
        categoria_nuance_tecnica="N/A",
        classificacao_terminal="SHORT-CIRCUIT",
        stack_base="unknown",
        tipo_integracao="Sidecar Efêmero",
        integracao_papel_exato="N/A",
        must_components="N/A",
        proposta_original_resumo="N/A",
        lente_a_sentido_ux="N/A (Short-Circuit)",
        lente_b_estrutura_arq="N/A (Short-Circuit)",
        lente_c_realidade_ops="N/A (Short-Circuit)",
        visao_do_enxame="N/A (Short-Circuit)",
        executive_verdict="Repositório inacessível ou morto. Abortado via Short-Circuit.",
        ouro_a_extrair="N/A",
        deep_pattern="N/A",
        acao_de_canibalizacao="No Absorption",
        transplantable_core="N/A",
        logic_math_heuristic="N/A",
        risco_principal="Repositório morto ou inacessível — inviável auditar o código.",
        risco_linha_vermelha="Repositório morto ou inacessível — inviável auditar o código.",
        observacoes="Short-Circuit da Fase 1.",
        real_structural_problem="N/A",
        bare_metal_fit="LOW",
        discipline_dependency="Crítica (Quebra sem Disciplina)",
        extractability_level="LOW",
        operability_level="LOW",
        where_ai_should_not_enter="N/A",
        do_not_absorb="Tudo",
        data_ultima_analise=datetime.now().isoformat(timespec='seconds'),
        analise_origem="SODA ETL V3 Auto",
        lote_id=lote_id,
    )
