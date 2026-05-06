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

    project_name: str = Field(description="OBRIGATÓRIO o formato 'Owner / Repository'. NUNCA apenas o nome do projeto.")
    declared_description: str = Field(description="PT-BR. A tradução literal do 'Elevator Pitch' do autor. Sem jargões SODA.")
    repo_url: str = Field(description="URL do repositório (Chave Primária).")
    score_final: float = Field(default=0.0, ge=0.0, le=10.0, description="Score final ponderado (0 a 10).")
    score_fit_geral_soda: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de fit geral no SODA (0 a 10).")
    score_philosophical_fit: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de fit filosófico (0 a 10).")
    score_bare_metal_fit: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de fit bare-metal (0 a 10).")
    score_architectural_extractability: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de extraibilidade arquitetural (0 a 10).")
    score_operability: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de operabilidade (0 a 10).")
    score_creep_risk: float = Field(default=0.0, ge=0.0, le=10.0, description="Score de risco de scope creep (0 a 10).")
    
    entropy_risk: Literal["LOW", "MEDIUM", "HIGH", "CRITICAL"] = Field(description="Risco de entropia (LOW, MEDIUM, HIGH, CRITICAL).")
    design_misuse_risk: str = Field(description="Risco de mau uso do design em Português.")
    intrinsic_ethics_risk: str = Field(description="Risco ético intrínseco em Português.")
    
    horizonte_extracao: Literal["IMEDIATO", "CURTO_PRAZO", "CURTO_MEDIO_PRAZO", "MEDIO_PRAZO", "LONGO_PRAZO", "REFERENCIAL_TEORICO"] = Field(description="Horizonte temporal para extração.")
    justificativa_decisao: str = Field(description="A defesa coesa que amarra o Score, os Riscos e a Classificação Terminal.")
    categoria_arquitetural: Literal["Canvas", "Interface", "Memória", "Roteamento", "Orquestração", "Segurança", "Infraestrutura", "Tooling"] = Field(description="Eixo único da categoria arquitetural.")
    categoria_nuance_tecnica: str = Field(description="1 linha curta detalhando a categoria macro (ex: 'Busca Híbrida BM25 + Vetor').")
    classificacao_terminal: Literal["STACK_CORE_PLANO_A1", "STACK_CORE_PLANO_A2", "STACK_CORE_PLANO_A3", "INTEGRATE_AS_COMPONENT", "ABSORB_PARTIALLY", "ABSORB_CONCEPT", "USE_AS_INSPIRATION_ONLY", "REJECT", "SHORT-CIRCUIT"] = Field(description="A classificação terminal e definitiva.")
    
    stack_base: str = Field(description="Lista curta separada por vírgulas do ecossistema original (ex: 'Node.js, Electron, React' ou 'Rust, Tauri, SQLite').")
    tipo_integracao: Literal["Biblioteca / Crate Nativa", "Sidecar Efêmero", "Daemon / Background Service", "App Nativo / CLI Independente", "Middleware / Proxy"] = Field(description="O formato exato de integração.")
    integracao_papel_exato: str = Field(description="1 linha explicando a missão da integração no SODA.")
    must_components: str = Field(description="Lista de compras separada por vírgulas do que EXATAMENTE vamos canibalizar.")
    proposta_original_resumo: str = Field(description="A explicação neutra e técnica do produto, retirando o 'hype' de marketing.")
    
    lente_a_sentido_ux: str = Field(description="Max 5 linhas. Focar no 'UAU'. Como encanta o humano? Qual a novidade de mercado? Como essa sacada brilha no SODA?")
    lente_b_estrutura_arq: str = Field(description="Max 5 linhas. Focar na elegância da engenharia, padrões geniais, matemática e o esforço para transpilar para Rust.")
    lente_c_realidade_ops: str = Field(description="Max 5 linhas. Avaliar atrito 24/7, dependência de nuvem, licenças tóxicas e o 'DevOps Babá' necessário.")
    visao_do_enxame: str = Field(description="3 a 5 linhas. Síntese do conflito entre as 3 Lentes, encerrando com o direcionamento do Orquestrador.")
    executive_verdict: str = Field(max_length=1000, description="EXIGE a aplicação do 'Materialismo Dialético' em 5 a 8 linhas: Tese (o valor brilhante humano), Antítese (o atrito/limitação de hardware local), Síntese (a decisão final do Arquiteto SODA).")
    
    ouro_a_extrair: str = Field(description="O valor intangível, a sacada genial abstrata do projeto.")
    deep_pattern: str = Field(description="Padrão arquitetural profundo identificado em Português.")
    acao_de_canibalizacao: Literal["Data Model / Schema", "Prompt / Heuristic Seed", "Protocol / Standard", "Concept", "UX Pattern", "Canvas Refinement", "New Canvas", "Cognitive Layer", "Infra Capability", "Technical Runtime", "Sandbox", "Plugin", "External Contract", "No Absorption"] = Field(description="O que exatamente está sendo canibalizado.")
    transplantable_core: str = Field(description="O órgão físico/mecânico a ser extraído (ex: 'Motor de parsing AST').")
    logic_math_heuristic: str = Field(description="A matemática bruta, heurística ou algoritmo exato por trás da mágica.")
    risco_principal: str = Field(description="Uma única frase categórica justificando o risco mais alto identificado na ferramenta.")
    risco_linha_vermelha: str = Field(description="Restrição absoluta. DEVE começar com 'NUNCA FAZER...' ou 'PROIBIDO...'.")
    observacoes: str = Field(description="Máximo de 2 linhas para notas de rodapé operacionais (ex: 'Licença MIT, mas requer API Key').")
    real_structural_problem: str = Field(description="O problema concreto + a abstração (ex: 'Capacidade infra-semântica de extração de dados brutos de PDFs').")
    
    bare_metal_fit: Literal["LOW", "MEDIUM", "HIGH", "EXCELLENT"] = Field(description="Fit bare-metal.")
    discipline_dependency: Literal["Nenhuma (Automação Invisível)", "Baixa (Tolerante a Erros)", "Média (Conformidade Básica)", "Alta (Exige Mudança de Hábito)", "Crítica (Quebra sem Disciplina)"] = Field(description="Nível de disciplina técnica exigida.")
    extractability_level: Literal["LOW", "MEDIUM", "HIGH", "EXCELLENT"] = Field(description="Nível de facilidade de extração.")
    operability_level: Literal["LOW", "MEDIUM", "HIGH", "EXCELLENT"] = Field(description="Nível de facilidade de operação local.")
    
    where_ai_should_not_enter: str = Field(description="2 a 4 linhas mapeando as fronteiras determinísticas onde a estocasticidade da IA é proibida.")
    do_not_absorb: str = Field(description="Lista exata do lixo a ser amputado (ex: 'Descartar Docker, servidor Express e React').")
    data_ultima_analise: str = Field(description="Data da análise (ISO-8601).")
    analise_origem: str = Field(description="Origem da análise.")
    lote_id: str = Field(description="ID do Lote de processamento.")


# ---------------------------------------------------------------------------
# Helpers de Classificação (usados na Fase 3)
# ---------------------------------------------------------------------------

def classificar(score_total: float) -> Literal["STACK_CORE_PLANO_A1", "STACK_CORE_PLANO_A2", "STACK_CORE_PLANO_A3", "INTEGRATE_AS_COMPONENT", "ABSORB_PARTIALLY", "ABSORB_CONCEPT", "USE_AS_INSPIRATION_ONLY", "REJECT", "SHORT-CIRCUIT"]:
    """Aplica as regras canônicas de classificação terminal SODA V3."""
    if score_total >= 9.5:
        return "STACK_CORE_PLANO_A1"
    if score_total >= 9.0:
        return "STACK_CORE_PLANO_A2"
    if score_total >= 8.5:
        return "INTEGRATE_AS_COMPONENT"
    if score_total >= 6.5:
        return "ABSORB_CONCEPT"
    if score_total >= 4.0:
        return "USE_AS_INSPIRATION_ONLY"
    return "REJECT"

