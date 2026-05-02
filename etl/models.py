"""
models.py — Contratos Pydantic das 3 Fases do ETL Orchestrator.

T-02: Contratos de Dados (lidos antes de qualquer código de produção nas phases.py).
"""
from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field


# ---------------------------------------------------------------------------
# Fase 1 — Kimi K2 (Triagem e Contexto)
# ---------------------------------------------------------------------------

class RepoContext(BaseModel):
    """Saída estruturada da Fase 1 (Kimi K2 via OpenRouter)."""

    primary_language: str = "unknown"
    domain_hint: str = "unknown"          # "web-framework" | "cli-tool" | "ml-lib" | "unknown"
    summary: str = Field(default="", max_length=300)
    has_rust_components: bool = False
    has_wasm_targets: bool = False
    estimated_complexity: Literal["LOW", "MED", "HIGH"] = "MED"


# ---------------------------------------------------------------------------
# Fase 2 — Map-Reduce Socrático (3 Lentes SODA via OpenRouter)
# ---------------------------------------------------------------------------

class LenteOutput(BaseModel):
    """Saída estruturada de uma Lente individual do Enxame."""

    raw_analysis: str = Field(default="", max_length=800)
    score_parcial: float = Field(default=0.0, ge=0.0, le=10.0)
    flags: list[str] = Field(default_factory=list)


class SwarmResult(BaseModel):
    """
    Agregado das 3 Lentes após asyncio.gather().
    lente_* pode ser None se aquela chamada OpenRouter falhou.
    """

    lente_a: LenteOutput | None = None   # anthropic/claude-opus-4.7  — UX/Produto
    lente_b: LenteOutput | None = None   # deepseek/deepseek-v4-pro   — Arquitetura
    lente_c: LenteOutput | None = None   # zhipuai/glm-5              — Operação
    lentes_disponiveis: int = Field(default=0, ge=0, le=3)


# ---------------------------------------------------------------------------
# Fase 3 — Pydantic AI (Síntese + Validação JSON Estrito)
# ---------------------------------------------------------------------------

class RepoHeuristic(BaseModel):
    """
    Schema canônico final com 45 colunas exatas (SODA V3).
    A chave primária natural é repo_url.
    """

    project_name: str = Field(description="Nome do projeto.")
    declared_description: str = Field(description="Descrição declarada ou resumo do projeto. A descrição original DEVE ser traduzida obrigatoriamente para o Português impecável e técnico.")
    repo_url: str = Field(description="URL do repositório (Chave Primária).")
    score_final: float = Field(default=0.0, ge=0.0, le=10.0, description="Score final ponderado (0 a 10).")
    score_fit_geral_soda: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de fit geral no SODA (0 a 10).")
    score_philosophical_fit: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de fit filosófico (0 a 10).")
    score_bare_metal_fit: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de fit bare-metal (0 a 10).")
    score_architectural_extractability: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de extraibilidade arquitetural (0 a 10).")
    score_operability: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de operabilidade (0 a 10).")
    score_creep_risk: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de risco de scope creep (0 a 10).")
    entropy_risk: str = Field(description="Risco de entropia (Low, Medium, High) justificado em Português.")
    design_misuse_risk: str = Field(description="Risco de mau uso do design em Português.")
    intrinsic_ethics_risk: str = Field(description="Risco ético intrínseco em Português.")
    horizonte_extracao: Literal["MVP", "Post-MVP", "Advanced Phase", "Research Only", "Never"]
    justificativa_decisao: str = Field(description="Explicar o porquê do veredito acima em Português.")
    categoria_arquitetural: Literal["New Canvas", "Refinement of Canvas", "Feature Set", "Cognitive Layer", "Infra-Semantic Capability", "Capability Layer", "Technical Infrastructure", "Tooling / Support", "Pattern Only", "Domain App"]
    categoria_nuance_tecnica: str = Field(description="Nuance técnica da categoria arquitetural em Português.")
    classificacao_terminal: Literal["Integrate as Component", "Absorb Partially", "Absorb Concept", "Use as Inspiration Only", "Keep Under Observation", "Needs More Research", "Reject", "TRIAGEM"]
    stack_base: str = Field(description="Linguagem ou stack base principal.")
    tipo_integracao: Literal["Source", "Sensor", "Middle Layer", "Destination", "Engine", "Sandbox", "Developer Tooling", "User Tool", "Agent Tool", "Knowledge Sink", "Knowledge Source", "Support", "Guardrail", "Interface", "Runtime"]
    integracao_papel_exato: str = Field(description="Papel exato da integração na arquitetura em Português.")
    must_components: str = Field(description="Componentes obrigatórios para extração em Português.")
    proposta_original_resumo: str = Field(description="Resumo da proposta original do repositório em Português.")
    lente_a_sentido_ux: str = Field(description="Análise da Lente A (UX/Produto) em Português.")
    lente_b_estrutura_arq: str = Field(description="Análise da Lente B (Arquitetura) em Português.")
    lente_c_realidade_ops: str = Field(description="Análise da Lente C (Operação/Ops) em Português.")
    executive_verdict: str = Field(description="Veredito executivo final em Português.")
    ouro_a_extrair: str = Field(description="O ouro ou núcleo de valor a ser extraído em Português.")
    deep_pattern: str = Field(description="Padrão arquitetural profundo identificado em Português.")
    acao_de_canibalizacao: Literal["Integrate As-Is", "Integrate Partially", "Reimplement Internally", "Use as External Contract", "Absorb as Pattern Only", "Absorb as Philosophy Only", "Do Not Absorb"]
    transplantable_core: str = Field(description="Núcleo lógico/matemático transplantável em Português.")
    logic_math_heuristic: str = Field(description="Heurística lógica ou matemática principal em Português.")
    risco_principal: Literal["Low", "Medium", "High", "Critical"]
    risco_linha_vermelha: str = Field(description="Descrição da linha vermelha de risco em Português.")
    observacoes: str = Field(description="Observações adicionais em Português.")
    real_structural_problem: str = Field(description="Problema estrutural real resolvido pelo repositório em Português.")
    bare_metal_fit: Literal["Low", "Medium", "High", "Excellent"]
    discipline_dependency: str = Field(description="Dependência de disciplina técnica ou conhecimento específico em Português.")
    extractability_level: Literal["High", "Medium", "Low"]
    operability_level: Literal["Very Low", "Low", "Medium", "High", "Excellent"]
    where_ai_should_not_enter: str = Field(description="Áreas onde a IA não deve interferir em Português.")
    do_not_absorb: str = Field(description="O que NÃO deve ser absorvido do repositório em Português.")
    data_ultima_analise: str = Field(description="Data da análise (ISO-8601).")
    analise_origem: str = Field(description="Origem da análise.")
    lote_id: str = Field(description="ID do Lote de processamento.")


# ---------------------------------------------------------------------------
# Helpers de Classificação (usados na Fase 3)
# ---------------------------------------------------------------------------

def classificar(score_total: float) -> Literal["Integrate as Component", "Absorb Partially", "Absorb Concept", "Use as Inspiration Only", "Keep Under Observation", "Needs More Research", "Reject", "TRIAGEM"]:
    """Aplica as regras canônicas de classificação terminal SODA V3."""
    if score_total >= 8.5:
        return "Integrate as Component"
    if score_total >= 6.5:
        return "Absorb Concept"
    if score_total >= 4.0:
        return "Keep Under Observation"
    return "Reject"

