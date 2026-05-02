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
import sqlite3
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
    "You are a senior software architect. Analyze the given GitHub repository URL "
    "and return a JSON object with these exact keys: "
    "primary_language (str), domain_hint (str: one of web-framework/cli-tool/ml-lib/"
    "embedded/database/unknown), summary (str, max 300 chars), "
    "has_rust_components (bool), has_wasm_targets (bool), "
    "estimated_complexity (str: LOW/MED/HIGH). Return ONLY valid JSON."
)

_LENTE_A_SYSTEM = (
    "You are a UX/Product specialist evaluating a GitHub repository for integration into "
    "a bare-metal Rust ecosystem (SODA). "
    "CRITICAL RULE: PROIBIDO ler ou referenciar dados antigos. Evite Anchoring Bias. "
    "DIRETRIZ DE EQUILÍBRIO: O SODA odeia runtimes tóxicos (Node.js/Python), mas AMA abstrações e heurísticas geniais. Se a linguagem original for tóxica, puna severamente o 'bare_metal_fit' e a 'operability_level' (Nota 0 ou 1). NO ENTANTO, se a Visão de Produto (UX), a Lógica Matemática ou o Paradigma forem excelentes, exalte-os! Dê notas 8, 9 ou 10 para o valor estrutural. A solução não deve ser rejeitada cegamente; a 'acao_de_canibalizacao' deve refletir 'Reimplement Internally' (transpilar para Rust) ou 'Integrate Partially' (via Sidecar Efêmero em Wasmtime/Micro-VM isolado). Entenda o 'PORQUÊ' da ferramenta existir. "
    "DIRETRIZ DE IDIOMA: Toda a sua resposta estrutural e argumentativa DEVE ser rigorosamente em Português do Brasil. É terminantemente proibido redigir a análise em Inglês, exceto para nomes de arquivos ou literais de código. "
    "Analise o contexto fresco da Fase 1 e extraia proativamente insights rigorosos "
    "para preencher os 45 campos estruturais e descritivos do schema (focando em UX, "
    "must_components, proposta_original, etc). "
    "Return JSON with keys: raw_analysis (str, max 800 chars), score_parcial (float 0-10), "
    "flags (list[str], e.g. ['has_docs','no_examples','poor_api']). ONLY valid JSON."
)

_LENTE_B_SYSTEM = (
    "You are a systems architect evaluating a GitHub repository for Rust integration potential. "
    "CRITICAL RULE: PROIBIDO ler ou referenciar dados antigos. Evite Anchoring Bias. "
    "DIRETRIZ DE EQUILÍBRIO: O SODA odeia runtimes tóxicos (Node.js/Python), mas AMA abstrações e heurísticas geniais. Se a linguagem original for tóxica, puna severamente o 'bare_metal_fit' e a 'operability_level' (Nota 0 ou 1). NO ENTANTO, se a Visão de Produto (UX), a Lógica Matemática ou o Paradigma forem excelentes, exalte-os! Dê notas 8, 9 ou 10 para o valor estrutural. A solução não deve ser rejeitada cegamente; a 'acao_de_canibalizacao' deve refletir 'Reimplement Internally' (transpilar para Rust) ou 'Integrate Partially' (via Sidecar Efêmero em Wasmtime/Micro-VM isolado). Entenda o 'PORQUÊ' da ferramenta existir. "
    "DIRETRIZ DE IDIOMA: Toda a sua resposta estrutural e argumentativa DEVE ser rigorosamente em Português do Brasil. É terminantemente proibido redigir a análise em Inglês, exceto para nomes de arquivos ou literais de código. "
    "Analise o contexto fresco da Fase 1 e extraia proativamente insights rigorosos "
    "para preencher os 45 campos estruturais e descritivos do schema (focando em "
    "categoria_arquitetural, tipo_integracao, deep_pattern, transplantable_core, etc). "
    "Return JSON with keys: raw_analysis (str, max 800 chars), score_parcial (float 0-10), "
    "flags (list[str], e.g. ['has_tokio','tight_coupling','ffi_friendly']). ONLY valid JSON."
)

_LENTE_C_SYSTEM = (
    "You are a DevOps/operations specialist evaluating a GitHub repository for long-term "
    "maintainability in a bare-metal embedded context. "
    "CRITICAL RULE: PROIBIDO ler ou referenciar dados antigos. Evite Anchoring Bias. "
    "DIRETRIZ DE EQUILÍBRIO: O SODA odeia runtimes tóxicos (Node.js/Python), mas AMA abstrações e heurísticas geniais. Se a linguagem original for tóxica, puna severamente o 'bare_metal_fit' e a 'operability_level' (Nota 0 ou 1). NO ENTANTO, se a Visão de Produto (UX), a Lógica Matemática ou o Paradigma forem excelentes, exalte-os! Dê notas 8, 9 ou 10 para o valor estrutural. A solução não deve ser rejeitada cegamente; a 'acao_de_canibalizacao' deve refletir 'Reimplement Internally' (transpilar para Rust) ou 'Integrate Partially' (via Sidecar Efêmero em Wasmtime/Micro-VM isolado). Entenda o 'PORQUÊ' da ferramenta existir. "
    "DIRETRIZ DE IDIOMA: Toda a sua resposta estrutural e argumentativa DEVE ser rigorosamente em Português do Brasil. É terminantemente proibido redigir a análise em Inglês, exceto para nomes de arquivos ou literais de código. "
    "Analise o contexto fresco da Fase 1 e extraia proativamente insights rigorosos "
    "para preencher os 45 campos estruturais e descritivos do schema (focando em "
    "risco_principal, operability_level, entropy_risk, bare_metal_fit, etc). "
    "Return JSON with keys: raw_analysis (str, max 800 chars), score_parcial (float 0-10), "
    "flags (list[str], e.g. ['viral_license','no_ci','stale_repo','heavy_deps']). ONLY valid JSON."
)

_SYNTHESIS_SYSTEM = (
    "You are a CTO synthesizing three specialist analyses of a GitHub repository "
    "into a concise executive verdict and deep architectural classification. "
    "Return a JSON object containing exactly these string fields (em Português): "
    "executive_verdict, entropy_risk, design_misuse_risk, intrinsic_ethics_risk, "
    "horizonte_extracao, justificativa_decisao, categoria_nuance_tecnica, "
    "stack_base, tipo_integracao, integracao_papel_exato, must_components, "
    "proposta_original_resumo, ouro_a_extrair, deep_pattern, acao_de_canibalizacao, "
    "transplantable_core, logic_math_heuristic, risco_principal, risco_linha_vermelha, "
    "observacoes, real_structural_problem, bare_metal_fit, discipline_dependency, "
    "extractability_level, operability_level, where_ai_should_not_enter, do_not_absorb. "
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
    Chama Kimi K2 via OpenRouter para extrair o contexto estruturado do repositório.
    Timeout: 30s. Fallback: RepoContext com domain_hint='unknown' em caso de falha.
    Nunca aborta o lote — registra o erro em etl_errors(fase=1) e prossegue.
    """
    user_msg = f"Analyze this GitHub repository and return the JSON: {repo_url}"
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
    readme_hint = (
        f"Repository: {repo_url}\n"
        f"Language: {ctx.primary_language} | Domain: {ctx.domain_hint}\n"
        f"Complexity: {ctx.estimated_complexity} | "
        f"Rust: {ctx.has_rust_components} | Wasm: {ctx.has_wasm_targets}\n"
        f"Summary: {ctx.summary}"
    )

    results = await asyncio.gather(
        _call_lente(_MODEL_LENS_UX,  _LENTE_A_SYSTEM, readme_hint, _HEADERS_HEAVY, "Lente-A-UX",  repo_id),
        _call_lente(_MODEL_LENS_ARQ, _LENTE_B_SYSTEM, readme_hint, _HEADERS_HEAVY, "Lente-B-ARQ", repo_id),
        _call_lente(_MODEL_LENS_OPS, _LENTE_C_SYSTEM, readme_hint, _HEADERS_HEAVY, "Lente-C-OPS", repo_id),
        return_exceptions=True,
    )

    lente_a, lente_b, lente_c = None, None, None
    lentes_ok = 0

    for i, (result, name) in enumerate(zip(results, ["Lente-A-UX", "Lente-B-ARQ", "Lente-C-OPS"]), start=1):
        if isinstance(result, Exception):
            logger.warning("[FASE-2][%s] %s falhou: %s", repo_id, name, result)
            log_error(conn, run_id, repo_id, fase=2, exc=result)
        else:
            if i == 1:
                lente_a = result
            elif i == 2:
                lente_b = result
            else:
                lente_c = result
            lentes_ok += 1

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
        entropy_risk=_get("entropy_risk", "Medium"),
        design_misuse_risk=_get("design_misuse_risk", "Medium"),
        intrinsic_ethics_risk=_get("intrinsic_ethics_risk", "Low"),
        horizonte_extracao=_get("horizonte_extracao", "Post-MVP") if _get("horizonte_extracao", "Post-MVP") in ["MVP", "Post-MVP", "Advanced Phase", "Research Only", "Never"] else "Post-MVP",
        justificativa_decisao=_get("justificativa_decisao", justificativa)[:400],
        categoria_arquitetural=categoria if categoria in ["New Canvas", "Refinement of Canvas", "Feature Set", "Cognitive Layer", "Infra-Semantic Capability", "Capability Layer", "Technical Infrastructure", "Tooling / Support", "Pattern Only", "Domain App"] else "Tooling / Support",
        categoria_nuance_tecnica=_get("categoria_nuance_tecnica", categoria),
        classificacao_terminal=classificacao,
        stack_base=_get("stack_base", ctx.primary_language),
        tipo_integracao=_get("tipo_integracao", "Support") if _get("tipo_integracao", "Support") in ["Source", "Sensor", "Middle Layer", "Destination", "Engine", "Sandbox", "Developer Tooling", "User Tool", "Agent Tool", "Knowledge Sink", "Knowledge Source", "Support", "Guardrail", "Interface", "Runtime"] else "Support",
        integracao_papel_exato=_get("integracao_papel_exato", "Pendente"),
        must_components=_get("must_components", "Pendente"),
        proposta_original_resumo=_get("proposta_original_resumo", ctx.summary[:200]),
        lente_a_sentido_ux=swarm.lente_a.raw_analysis[:600] if swarm.lente_a else "N/A",
        lente_b_estrutura_arq=swarm.lente_b.raw_analysis[:600] if swarm.lente_b else "N/A",
        lente_c_realidade_ops=swarm.lente_c.raw_analysis[:600] if swarm.lente_c else "N/A",
        executive_verdict=executive_verdict,
        ouro_a_extrair=_get("ouro_a_extrair", "Pendente"),
        deep_pattern=_get("deep_pattern", "Pendente"),
        acao_de_canibalizacao=_get("acao_de_canibalizacao", "Do Not Absorb") if _get("acao_de_canibalizacao", "Do Not Absorb") in ["Integrate As-Is", "Integrate Partially", "Reimplement Internally", "Use as External Contract", "Absorb as Pattern Only", "Absorb as Philosophy Only", "Do Not Absorb"] else "Do Not Absorb",
        transplantable_core=_get("transplantable_core", "Pendente"),
        logic_math_heuristic=_get("logic_math_heuristic", "Pendente"),
        risco_principal=_get("risco_principal", "Medium") if _get("risco_principal", "Medium") in ["Low", "Medium", "High", "Critical"] else "Medium",
        risco_linha_vermelha=_get("risco_linha_vermelha", "Pendente"),
        observacoes=_get("observacoes", "Nenhuma"),
        real_structural_problem=_get("real_structural_problem", "Pendente"),
        bare_metal_fit=_get("bare_metal_fit", "Medium") if _get("bare_metal_fit", "Medium") in ["Low", "Medium", "High", "Excellent"] else "Medium",
        discipline_dependency=_get("discipline_dependency", "Pendente"),
        extractability_level=_get("extractability_level", "Medium") if _get("extractability_level", "Medium") in ["High", "Medium", "Low"] else "Medium",
        operability_level=_get("operability_level", "Medium") if _get("operability_level", "Medium") in ["Very Low", "Low", "Medium", "High", "Excellent"] else "Medium",
        where_ai_should_not_enter=_get("where_ai_should_not_enter", "Pendente"),
        do_not_absorb=_get("do_not_absorb", "Pendente"),
        data_ultima_analise="2026-05-01",
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
        entropy_risk="High",
        design_misuse_risk="N/A",
        intrinsic_ethics_risk="N/A",
        horizonte_extracao="Never",
        justificativa_decisao="Repositório inacessível ou morto. Abortado via Short-Circuit na Fase 1.",
        categoria_arquitetural="Tooling / Support",
        categoria_nuance_tecnica="N/A",
        classificacao_terminal="Reject",
        stack_base="unknown",
        tipo_integracao="Support",
        integracao_papel_exato="N/A",
        must_components="N/A",
        proposta_original_resumo="N/A",
        lente_a_sentido_ux="N/A (Short-Circuit)",
        lente_b_estrutura_arq="N/A (Short-Circuit)",
        lente_c_realidade_ops="N/A (Short-Circuit)",
        executive_verdict="Repositório inacessível ou morto. Abortado via Short-Circuit.",
        ouro_a_extrair="N/A",
        deep_pattern="N/A",
        acao_de_canibalizacao="Do Not Absorb",
        transplantable_core="N/A",
        logic_math_heuristic="N/A",
        risco_principal="Critical",
        risco_linha_vermelha="Repositório morto ou inacessível — inviável auditar o código.",
        observacoes="Short-Circuit da Fase 1.",
        real_structural_problem="N/A",
        bare_metal_fit="Low",
        discipline_dependency="N/A",
        extractability_level="Low",
        operability_level="Very Low",
        where_ai_should_not_enter="N/A",
        do_not_absorb="Tudo",
        data_ultima_analise="2026-05-01",
        analise_origem="SODA ETL V3 Auto",
        lote_id=lote_id,
    )
