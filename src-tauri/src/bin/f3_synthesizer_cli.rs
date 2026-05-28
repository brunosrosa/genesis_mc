use std::io;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use chrono::{FixedOffset, Utc};
use genesis_mc_lib::cognition::synthesizer::{
    run_phase3_sgr, Block0Context, Phase3Config, Phase3Error, OFFICIAL_FORMATTER_MODEL,
};
use genesis_mc_lib::harvester::community::{CommunityMetaFetcher, RateLimiter};
use genesis_mc_lib::persist::ssot_injector::SsotInjector;
use reqwest::Client;
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{error, info};
use url::Url;

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::other("Falha ao resolver raiz do projeto"))
}

fn now_epoch_secs() -> io::Result<i64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::other(format!("Falha ao calcular timestamp atual: {}", e)))?
        .as_secs() as i64)
}

async fn try_fetch_github_latest_release_tag(repo_url: &str) -> Option<String> {
    let url = Url::parse(repo_url).ok()?;
    let allow_host_override = std::env::var("SODA_GITHUB_API_BASE_URL").is_ok();
    if url.host_str() != Some("github.com") && !allow_host_override {
        return None;
    }
    let mut segments = url
        .path_segments()
        .map(|parts| parts.collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.trim_end_matches(".git").to_string())
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return None;
    }
    let repo = segments.pop()?;
    let owner = segments.pop()?;

    let base = std::env::var("SODA_GITHUB_API_BASE_URL").unwrap_or_else(|_| "https://api.github.com".to_string());
    let endpoint = format!("{}/repos/{owner}/{repo}/releases/latest", base.trim_end_matches('/'));

    #[derive(Deserialize)]
    struct GithubRelease {
        tag_name: Option<String>,
    }

    let client = Client::builder()
        .user_agent("f3-synthesizer-cli/1.0")
        .build()
        .ok()?;
    let resp = client.get(&endpoint).send().await.ok()?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return None;
    }
    if !resp.status().is_success() {
        return None;
    }
    let release = resp.json::<GithubRelease>().await.ok()?;
    release
        .tag_name
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn sanitize_repo_id(repo_id: &str) -> String {
    repo_id
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}

fn now_brt_rfc3339() -> String {
    Utc::now()
        .with_timezone(&FixedOffset::west_opt(3 * 3600).unwrap())
        .to_rfc3339()
}

fn etl_report_path(root_dir: &Path, repo_id: &str) -> io::Result<PathBuf> {
    let dir = root_dir.join(".soda_scratchpad").join("reports");
    std::fs::create_dir_all(&dir)
        .map_err(|e| io::Error::other(format!("Falha ao criar reports_dir: {}", e)))?;

    let trimmed = repo_id.trim();
    let mut parts = trimmed.split('/').map(|s| s.trim()).filter(|s| !s.is_empty());
    let owner = parts.next().unwrap_or(trimmed);
    let repo = parts.next().unwrap_or(trimmed);
    Ok(dir.join(format!(
        "_ETL_REPORT_{}_{}.txt",
        sanitize_repo_id(owner),
        sanitize_repo_id(repo)
    )))
}

fn extract_total_cost_usd_from_lens_json(lens_json: &str) -> f64 {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(lens_json) else {
        return 0.0;
    };
    value.get("total_cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0)
}

#[derive(Debug, Clone)]
struct CliArgs {
    repo_id: String,
    e2e_full: bool,
}

fn parse_cli_args() -> CliArgs {
    let mut args = std::env::args();
    args.next();
    let mut repo_id = String::from("aaif-goose/goose");
    let mut e2e_full = false;
    while let Some(arg) = args.next() {
        if arg == "--repo" {
            if let Some(value) = args.next() {
                repo_id = value;
            }
            continue;
        }
        if arg == "--e2e-full" {
            e2e_full = true;
            continue;
        }
    }
    CliArgs {
        repo_id,
        e2e_full,
    }
}

fn get_first_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .and_then(|value| {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
    })
}

#[derive(Debug, Default, Clone)]
struct UsageTotals {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    total_cost_usd: f64,
}

struct OpenRouterFormatterClient {
    client: Client,
    base_url: String,
    api_key: String,
    usage: std::sync::Arc<std::sync::Mutex<UsageTotals>>,
}

impl OpenRouterFormatterClient {
    fn from_env() -> Result<Self, String> {
        let api_key = get_first_env(&[
            "OPENROUTER_API_HEAVY_KEY",
            "OPENROUTER_API_KEY",
            "OPENROUTER_API_FAST_KEY",
            "OPENROUTER_API_FREE_KEY",
        ])
        .ok_or_else(|| "OPENROUTER_API_HEAVY_KEY/OPENROUTER_API_KEY/OPENROUTER_API_FAST_KEY/OPENROUTER_API_FREE_KEY ausente".to_string())?;
        let base_url = std::env::var("OPENAI_BASE_URL")
            .map(|base| format!("{}/chat/completions", base.trim_end_matches('/')))
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1/chat/completions".to_string());

        Ok(Self {
            client: Client::new(),
            base_url,
            api_key,
            usage: std::sync::Arc::new(std::sync::Mutex::new(UsageTotals::default())),
        })
    }

    fn usage_totals(&self) -> UsageTotals {
        match self.usage.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => UsageTotals::default(),
        }
    }

        fn extract_openrouter_content(json: &Value) -> Option<String> {
        fn flatten(value: &Value) -> Option<String> {
            match value {
                Value::String(text) => {
                    let t = text.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                }
                Value::Array(parts) => {
                    let mut out = Vec::new();
                    for part in parts {
                        if let Some(text) = flatten(part) {
                            out.push(text);
                            continue;
                        }
                        if let Some(obj) = part.as_object() {
                            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                                let t = text.trim();
                                if !t.is_empty() {
                                    out.push(t.to_string());
                                    continue;
                                }
                            }
                            if let Some(text) = obj.get("content").and_then(|v| v.as_str()) {
                                let t = text.trim();
                                if !t.is_empty() {
                                    out.push(t.to_string());
                                    continue;
                                }
                            }
                            if let Some(text) = obj
                                .get("text")
                                .and_then(|v| v.get("value"))
                                .and_then(|v| v.as_str())
                            {
                                let t = text.trim();
                                if !t.is_empty() {
                                    out.push(t.to_string());
                                    continue;
                                }
                            }
                            if let Some(text) = obj
                                .get("content")
                                .and_then(|v| v.get("value"))
                                .and_then(|v| v.as_str())
                            {
                                let t = text.trim();
                                if !t.is_empty() {
                                    out.push(t.to_string());
                                    continue;
                                }
                            }
                        }
                    }
                    let joined = out.join("\n");
                    if joined.trim().is_empty() {
                        None
                    } else {
                        Some(joined)
                    }
                }
                Value::Object(obj) => {
                    if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                        let t = text.trim();
                        if !t.is_empty() {
                            return Some(t.to_string());
                        }
                    }
                    if let Some(text) = obj.get("content").and_then(|v| v.as_str()) {
                        let t = text.trim();
                        if !t.is_empty() {
                            return Some(t.to_string());
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        let choices = json.get("choices")?.as_array()?;
        let first = choices.first()?;
        if let Some(message) = first.get("message") {
            if let Some(content) = message.get("content") {
                if let Some(text) = flatten(content) {
                    return Some(text);
                }
            }
        }

        first.get("text").and_then(|v| flatten(v))
    }

    fn harvest_usage(&self, json: &Value) {
        let usage = &json["usage"];
        let prompt = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let completion = usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let total = usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cost = usage
            .get("total_cost")
            .or_else(|| usage.get("cost"))
            .or_else(|| usage.get("estimated_cost"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if let Ok(mut guard) = self.usage.lock() {
            guard.prompt_tokens = guard.prompt_tokens.saturating_add(prompt);
            guard.completion_tokens = guard.completion_tokens.saturating_add(completion);
            guard.total_tokens = guard.total_tokens.saturating_add(total);
            guard.total_cost_usd += cost;
        }
    }
}

fn parse_block_from_prompt(prompt: &str) -> Option<u8> {
    let first = prompt.lines().next()?.trim();
    let value = first.strip_prefix("BLOCK=")?.trim();
    value.parse::<u8>().ok()
}

fn response_format_for_block(block: u8) -> Value {
    fn strict_object(properties: serde_json::Map<String, Value>, required: Vec<&'static str>) -> Value {
        json!({
            "type": "object",
            "properties": Value::Object(properties),
            "required": required,
            "additionalProperties": false
        })
    }

    fn string_schema(max_len: u32) -> Value {
        json!({ "type": "string", "maxLength": max_len })
    }

    fn enum_schema(options: &[&str]) -> Value {
        json!({ "type": "string", "enum": options })
    }

    fn int_0_10_schema() -> Value {
        json!({ "type": "integer", "minimum": 0, "maximum": 10 })
    }

    fn envelope(fields_schema: Value, strict_fields: bool) -> Value {
        let mut props = serde_json::Map::new();
        props.insert("fields".to_string(), fields_schema);
        props.insert(
            "justifications".to_string(),
            json!({
                "type": "object",
                "additionalProperties": { "type": "string", "maxLength": 3000 }
            }),
        );
        let mut schema = strict_object(props, vec!["fields", "justifications"]);
        if let Some(obj) = schema.as_object_mut() {
            if strict_fields {
                // no-op, kept for readability: fields_schema already has additionalProperties=false
                let _ = obj;
            }
        }
        schema
    }

    let fields_schema = match block {
        1 => {
            let mut props = serde_json::Map::new();
            props.insert("proposta_original_resumo".to_string(), string_schema(3000));
            props.insert("declared_description_ptbr".to_string(), string_schema(3000));
            props.insert("visao_do_enxame".to_string(), string_schema(3000));
            props.insert("justificativa_decisao".to_string(), string_schema(3000));
            props.insert("executive_verdict".to_string(), string_schema(3000));
            props.insert("risco_principal".to_string(), string_schema(3000));
            props.insert("risco_linha_vermelha".to_string(), string_schema(3000));
            props.insert("observacoes".to_string(), string_schema(3000));
            strict_object(
                props,
                vec![
                    "proposta_original_resumo",
                    "declared_description_ptbr",
                    "visao_do_enxame",
                    "justificativa_decisao",
                    "executive_verdict",
                    "risco_principal",
                    "risco_linha_vermelha",
                    "observacoes",
                ],
            )
        }
        2 => {
            let mut props = serde_json::Map::new();
            props.insert("ouro_a_extrair".to_string(), string_schema(3000));
            props.insert("deep_pattern".to_string(), string_schema(3000));
            props.insert("transplantable_core".to_string(), string_schema(3000));
            props.insert("logic_math_heuristic".to_string(), string_schema(3000));
            props.insert("real_structural_problem".to_string(), string_schema(3000));
            props.insert("categoria_nuance_tecnica".to_string(), string_schema(1200));
            props.insert("integracao_papel_exato".to_string(), string_schema(1200));
            props.insert("must_components_prod_ux".to_string(), string_schema(3000));
            props.insert("must_components_arq".to_string(), string_schema(3000));
            props.insert("must_components_ops".to_string(), string_schema(3000));
            props.insert("detected_toxic_deps".to_string(), string_schema(3000));
            props.insert("do_not_absorb".to_string(), string_schema(3000));
            props.insert("where_ai_should_not_enter".to_string(), string_schema(3000));
            strict_object(
                props,
                vec![
                    "ouro_a_extrair",
                    "deep_pattern",
                    "transplantable_core",
                    "logic_math_heuristic",
                    "real_structural_problem",
                    "categoria_nuance_tecnica",
                    "integracao_papel_exato",
                    "must_components_prod_ux",
                    "must_components_arq",
                    "must_components_ops",
                    "detected_toxic_deps",
                    "do_not_absorb",
                    "where_ai_should_not_enter",
                ],
            )
        }
        3 => {
            let mut props = serde_json::Map::new();
            props.insert(
                "classificacao_terminal".to_string(),
                enum_schema(&[
                    "APROVADO_PARA_PRODUCAO",
                    "APROVADO_COM_RESSALVAS",
                    "REJEITADO_DESCARTE",
                ]),
            );
            props.insert(
                "acao_de_canibalizacao".to_string(),
                enum_schema(&["NENHUMA", "ABSORVER_LOGICA", "EXTRAIR_SCRIPTS"]),
            );
            props.insert(
                "categoria_arquitetural".to_string(),
                enum_schema(&[
                    "LIBRARY",
                    "FRAMEWORK",
                    "APPLICATION",
                    "TOOLING",
                    "INFRASTRUCTURE",
                    "RUNTIME",
                ]),
            );
            props.insert(
                "horizonte_extracao".to_string(),
                enum_schema(&["IMMEDIATE", "SHORT", "MEDIUM", "LONG", "VERY_LONG"]),
            );
            props.insert(
                "tipo_integracao".to_string(),
                enum_schema(&["INTEGRATE_AS_COMPONENT", "REIMPLEMENT_INTERNALLY", "REJECT"]),
            );
            props.insert(
                "capability_nature_primary".to_string(),
                enum_schema(&[
                    "LIBRARY",
                    "TOOLING",
                    "SERVICE",
                    "APPLICATION",
                    "SYSTEM",
                    "ALGORITHM",
                    "DATA_STRUCTURE",
                ]),
            );
            props.insert(
                "architectural_topology".to_string(),
                enum_schema(&[
                    "MODULAR",
                    "MONOLITH",
                    "LAYERED",
                    "MICROSERVICES",
                    "EVENT_DRIVEN",
                    "PIPELINE",
                    "PLUGIN",
                ]),
            );
            props.insert("temporal_stability".to_string(), enum_schema(&["STABLE", "EVOLVING"]));
            props.insert("bare_metal_fit".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert("extractability_level".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert("runtime_sovereignty_fit".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert("local_first_fit".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert(
                "adoptability_level".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert(
                "longitudinal_sustainability".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert(
                "maintenance_burden".to_string(),
                enum_schema(&["LOW", "MEDIUM", "HIGH", "VERY_HIGH"]),
            );
            props.insert(
                "onboarding_friction".to_string(),
                enum_schema(&["LOW", "MEDIUM", "HIGH", "VERY_HIGH"]),
            );
            props.insert(
                "observability_operational".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert(
                "recoverability_level".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert(
                "degradation_behavior".to_string(),
                enum_schema(&["GRACEFUL", "ACCEPTABLE", "FRAGILE", "CATASTROPHIC"]),
            );
            props.insert(
                "curation_burden".to_string(),
                enum_schema(&["LOW", "MEDIUM", "HIGH", "VERY_HIGH"]),
            );
            props.insert(
                "evolution_cost".to_string(),
                enum_schema(&["LOW", "MEDIUM", "HIGH", "VERY_HIGH"]),
            );
            props.insert("operability_level".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "EXCELLENT"]));
            props.insert("abandonment_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            props.insert(
                "time_to_first_clear_value".to_string(),
                enum_schema(&["IMMEDIATE", "SHORT", "MEDIUM", "LONG", "VERY_LONG"]),
            );
            props.insert(
                "imperfection_tolerance".to_string(),
                enum_schema(&["VERY_LOW", "LOW", "MEDIUM", "HIGH", "EXCELLENT"]),
            );
            props.insert("entropy_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            props.insert("design_misuse_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            props.insert("intrinsic_ethics_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            props.insert(
                "discipline_dependency".to_string(),
                enum_schema(&["NENHUMA", "BAIXA", "MEDIA", "ALTA", "CRITICA"]),
            );
            props.insert("regulatory_risk".to_string(), enum_schema(&["LOW", "MEDIUM", "HIGH", "CRITICAL"]));
            strict_object(
                props,
                vec![
                    "classificacao_terminal",
                    "acao_de_canibalizacao",
                    "categoria_arquitetural",
                    "horizonte_extracao",
                    "tipo_integracao",
                    "capability_nature_primary",
                    "architectural_topology",
                    "temporal_stability",
                    "bare_metal_fit",
                    "extractability_level",
                    "runtime_sovereignty_fit",
                    "local_first_fit",
                    "adoptability_level",
                    "longitudinal_sustainability",
                    "maintenance_burden",
                    "onboarding_friction",
                    "observability_operational",
                    "recoverability_level",
                    "degradation_behavior",
                    "curation_burden",
                    "evolution_cost",
                    "operability_level",
                    "abandonment_risk",
                    "time_to_first_clear_value",
                    "imperfection_tolerance",
                    "entropy_risk",
                    "design_misuse_risk",
                    "intrinsic_ethics_risk",
                    "discipline_dependency",
                    "regulatory_risk",
                ],
            )
        }
        4 => {
            let mut props = serde_json::Map::new();
            props.insert("score_philosophical_fit".to_string(), int_0_10_schema());
            props.insert("score_bare_metal_fit".to_string(), int_0_10_schema());
            props.insert("score_architectural_extractability".to_string(), int_0_10_schema());
            props.insert("score_operability".to_string(), int_0_10_schema());
            props.insert("score_creep_risk".to_string(), int_0_10_schema());
            props.insert("score_runtime_sovereignty".to_string(), int_0_10_schema());
            props.insert("score_model_logic_value".to_string(), int_0_10_schema());
            props.insert("score_ethics_safety".to_string(), int_0_10_schema());
            props.insert("score_intrinsic_risk".to_string(), int_0_10_schema());
            strict_object(
                props,
                vec![
                    "score_philosophical_fit",
                    "score_bare_metal_fit",
                    "score_architectural_extractability",
                    "score_operability",
                    "score_creep_risk",
                    "score_runtime_sovereignty",
                    "score_model_logic_value",
                    "score_ethics_safety",
                    "score_intrinsic_risk",
                ],
            )
        }
        _ => {
            let mut props = serde_json::Map::new();
            props.insert("note".to_string(), string_schema(200));
            strict_object(props, vec!["note"])
        }
    };

    let schema = envelope(fields_schema, true);
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": format!("soda_f3_block_{block}"),
            "strict": true,
            "schema": schema
        }
    })
}

fn example_output_for_block(block: u8) -> Value {
    let fields = match block {
        1 => json!({
            "proposta_original_resumo": "",
            "declared_description_ptbr": "",
            "visao_do_enxame": "",
            "justificativa_decisao": "",
            "executive_verdict": "",
            "risco_principal": "",
            "risco_linha_vermelha": "",
            "observacoes": ""
        }),
        2 => json!({
            "ouro_a_extrair": "",
            "deep_pattern": "",
            "transplantable_core": "",
            "logic_math_heuristic": "",
            "real_structural_problem": "",
            "categoria_nuance_tecnica": "",
            "integracao_papel_exato": "",
            "must_components_prod_ux": "",
            "must_components_arq": "",
            "must_components_ops": "",
            "detected_toxic_deps": "",
            "do_not_absorb": "",
            "where_ai_should_not_enter": ""
        }),
        3 => json!({
            "classificacao_terminal": "APROVADO_COM_RESSALVAS",
            "acao_de_canibalizacao": "NENHUMA",
            "categoria_arquitetural": "TOOLING",
            "horizonte_extracao": "SHORT",
            "tipo_integracao": "INTEGRATE_AS_COMPONENT",
            "capability_nature_primary": "TOOLING",
            "architectural_topology": "MODULAR",
            "temporal_stability": "EVOLVING",
            "bare_metal_fit": "MEDIUM",
            "extractability_level": "MEDIUM",
            "runtime_sovereignty_fit": "MEDIUM",
            "local_first_fit": "MEDIUM",
            "adoptability_level": "MEDIUM",
            "longitudinal_sustainability": "MEDIUM",
            "maintenance_burden": "MEDIUM",
            "onboarding_friction": "MEDIUM",
            "observability_operational": "MEDIUM",
            "recoverability_level": "MEDIUM",
            "degradation_behavior": "ACCEPTABLE",
            "curation_burden": "MEDIUM",
            "evolution_cost": "MEDIUM",
            "operability_level": "MEDIUM",
            "abandonment_risk": "MEDIUM",
            "time_to_first_clear_value": "SHORT",
            "imperfection_tolerance": "MEDIUM",
            "entropy_risk": "MEDIUM",
            "design_misuse_risk": "MEDIUM",
            "intrinsic_ethics_risk": "MEDIUM",
            "discipline_dependency": "MEDIA",
            "regulatory_risk": "MEDIUM"
        }),
        4 => json!({
            "score_philosophical_fit": 0,
            "score_bare_metal_fit": 0,
            "score_architectural_extractability": 0,
            "score_operability": 0,
            "score_creep_risk": 0,
            "score_runtime_sovereignty": 0,
            "score_model_logic_value": 0,
            "score_ethics_safety": 0,
            "score_intrinsic_risk": 0
        }),
        _ => json!({ "note": "" }),
    };

    json!({
        "fields": fields,
        "justifications": {}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_for_block3_has_enums_and_is_strict() {
        let rf = response_format_for_block(3);
        assert_eq!(rf.get("type").and_then(|v| v.as_str()), Some("json_schema"));
        let schema = rf
            .get("json_schema")
            .and_then(|v| v.get("schema"))
            .and_then(|v| v.as_object())
            .unwrap();
        assert_eq!(
            schema
                .get("additionalProperties")
                .and_then(|v| v.as_bool()),
            Some(false)
        );
        let fields = schema
            .get("properties")
            .and_then(|v| v.get("fields"))
            .and_then(|v| v.get("properties"))
            .and_then(|v| v.as_object())
            .unwrap();
        let ct = fields.get("classificacao_terminal").unwrap();
        let opts = ct.get("enum").and_then(|v| v.as_array()).unwrap();
        assert!(opts.iter().any(|v| v.as_str() == Some("APROVADO_PARA_PRODUCAO")));
    }
}

impl genesis_mc_lib::cognition::synthesizer::FormatterClient for OpenRouterFormatterClient {
    fn format<'a>(
        &'a self,
        model: &'a str,
        prompt: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let block = parse_block_from_prompt(prompt).unwrap_or(0);
            let example = example_output_for_block(block);
            let mut user_prompt = prompt.to_string();
            user_prompt.push_str("\n\nExample Output (JSON)\n");
            user_prompt.push_str(
                &serde_json::to_string_pretty(&example)
                    .unwrap_or_else(|_| r#"{"fields":{},"justifications":{}}"#.to_string()),
            );
            let body = json!({
                "model": model,
                "messages": [
                    {
                        "role": "system",
                        "content": "Responda SOMENTE com JSON válido (sem markdown, sem texto extra)."
                    },
                    {
                        "role": "user",
                        "content": user_prompt
                    }
                ],
                "temperature": 0.0,
                "max_tokens": 16000,
                "reasoning_effort": "xhigh",
                "response_format": response_format_for_block(block)
            });

            let response = self
                .client
                .post(&self.base_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Erro de rede: {}", e))?;

            let status = response.status();
            let raw = response.text().await.map_err(|e| e.to_string())?;
            if !status.is_success() {
                return Err(format!("HTTP {}: {}", status.as_u16(), raw));
            }

            let envelope: Value = serde_json::from_str(&raw)
                .map_err(|e| format!("Envelope JSON inválido do OpenRouter: {}", e))?;
            self.harvest_usage(&envelope);
            Self::extract_openrouter_content(&envelope).ok_or_else(|| "Resposta vazia do OpenRouter".to_string())
        })
    }
}

fn fetch_debates(conn: &Connection, repo_id: &str) -> io::Result<(String, String, String)> {
    conn.query_row(
        "SELECT lens_a_json, lens_b_json, lens_c_json
         FROM debates_enxame
         WHERE repo_id = ?1
         LIMIT 1",
        params![repo_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .map_err(|e| io::Error::other(format!("Debates da F2 (Enxame Cognitivo) ausentes em debates_enxame para {}: {}", repo_id, e)))
}

fn fetch_repo_core(conn: &Connection, repo_id: &str) -> io::Result<(String, String)> {
    conn.query_row(
        "SELECT lote_id, repo_url
         FROM repositorios
         WHERE project_name = ?1
         LIMIT 1",
        params![repo_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .map_err(|e| io::Error::other(format!("Metadados base ausentes em repositorios para {}: {}", repo_id, e)))
}

fn try_fetch_repositorios_release_info(
    conn: &Connection,
    repo_id: &str,
) -> (Option<String>, Option<String>) {
    let mut stmt = match conn.prepare(
        "SELECT repo_version, ultima_versao_online
         FROM repositorios
         WHERE project_name = ?1
         LIMIT 1",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return (None, None),
    };
    let row: (Option<String>, Option<String>) = match stmt.query_row(params![repo_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    }) {
        Ok(value) => value,
        Err(_) => return (None, None),
    };
    let repo_version = row
        .0
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let ultima_versao_online = row
        .1
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    (repo_version, ultima_versao_online)
}

fn try_fetch_repo_heuristics_seed(conn: &Connection, repo_id: &str) -> Option<(String, String, String, String, String)> {
    let mut stmt = conn
        .prepare(
            "SELECT repo_version, ultima_versao_online, licenca, stack_base, declared_description
             FROM repo_heuristics
             WHERE project_name = ?1
             LIMIT 1",
        )
        .ok()?;
    stmt.query_row(params![repo_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })
    .ok()
}

fn is_unknown_like(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    lower == "unknown" || lower == "desconhecido" || lower == "n/a"
}

fn fetch_raw_artifact_text(conn: &Connection, repo_id: &str, artifact_type: &str) -> Option<String> {
    conn.query_row(
        "SELECT CAST(payload_blob AS TEXT)
         FROM artefatos_brutos
         WHERE repo_id = ?1 AND artifact_type = ?2
         ORDER BY artifact_id DESC
         LIMIT 1",
        params![repo_id, artifact_type],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
}

fn derive_stack_base_from_manifest_blob(text: &str) -> Option<String> {
    for line in text.lines().take(20) {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("stack_base:") else {
            continue;
        };
        let val = rest.trim();
        if val.is_empty() {
            return None;
        }
        if is_unknown_like(val) {
            return None;
        }
        return Some(val.to_string());
    }
    None
}

fn derive_license_from_community_meta_json(text: &str) -> Option<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return None;
    };
    value
        .get("licenca")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !is_unknown_like(s))
}

fn derive_license_from_readme(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("mit license") || (lower.contains("license") && lower.contains("mit")) {
        return Some("MIT".to_string());
    }
    if lower.contains("apache license") || lower.contains("apache-2.0") {
        return Some("Apache-2.0".to_string());
    }
    if lower.contains("gnu general public license") || lower.contains("gpl") {
        return Some("GPL".to_string());
    }
    None
}

fn derive_declared_description_from_readme(text: &str) -> Option<String> {
    let mut fallback_heading: Option<String> = None;
    for line in text.lines().take(120) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("<") {
            continue;
        }
        if trimmed.starts_with('#') {
            if fallback_heading.is_none() {
                let heading = trimmed.trim_start_matches('#').trim();
                if !heading.is_empty() {
                    fallback_heading = Some(heading.to_string());
                }
            }
            continue;
        }
        let cleaned = trimmed.replace("**", "").replace('`', "");
        let candidate = cleaned.trim();
        if candidate.is_empty() {
            continue;
        }
        let max_chars = 2000_usize;
        let mut prefix = candidate.chars().take(max_chars).collect::<String>();
        if candidate.chars().count() > max_chars {
            while !prefix.is_empty() {
                let last = prefix.chars().last().unwrap_or(' ');
                if last.is_whitespace()
                    || last == '.'
                    || last == ','
                    || last == ';'
                    || last == ':'
                    || last == ')'
                    || last == ']'
                {
                    break;
                }
                prefix.pop();
            }
            prefix = prefix.trim_end().to_string();
        }
        if !prefix.is_empty() {
            return Some(prefix);
        }
    }
    fallback_heading
}

fn call_mcp(tool_name: &str, arguments: Value) -> io::Result<Value> {
    use std::process::{Command, Stdio};

    let creds = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .map_err(|_| io::Error::other("Missing GOOGLE_APPLICATION_CREDENTIALS"))?;

    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "phase3-4-cli", "version": "1.0.0" }
        }
    });
    let initialized_notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    });

    let mut child = Command::new("mcp-google-sheets")
        .env("GOOGLE_APPLICATION_CREDENTIALS", creds)
        .env("UV_NO_PROGRESS", "1")
        .env("UV_QUIET", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| io::Error::other(format!("Falha ao spawnar mcp-google-sheets: {}", e)))?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().ok_or_else(|| io::Error::other("stdin indisponível"))?;
        writeln!(stdin, "{}", init_req)?;
        writeln!(stdin, "{}", initialized_notif)?;
        writeln!(stdin, "{}", mcp_request)?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| io::Error::other(format!("Falha ao aguardar mcp-google-sheets: {}", e)))?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "mcp-google-sheets falhou. Exit {}. STDERR: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    for line in stdout_str.lines().rev() {
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            if value.get("id").and_then(|v| v.as_i64()) == Some(1) {
                if value.get("error").is_some() {
                    return Err(io::Error::other(format!("MCP retornou erro: {}", value)));
                }
                if let Some(result) = value.get("result") {
                    return Ok(normalize_mcp_tool_result(result.clone()));
                }
            }
        }
    }

    Err(io::Error::other("Resposta MCP não encontrada no stdout"))
}

fn normalize_mcp_tool_result(result: Value) -> Value {
    let content = match result.get("content").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return result,
    };

    for item in content {
        if let Some(json_val) = item.get("json") {
            return json_val.clone();
        }
        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
            if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                return parsed;
            }
        }
    }

    result
}

fn extract_values_2d(result: &Value) -> Option<Vec<Vec<String>>> {
    if let Some(values) = result.get("values").and_then(|v| v.as_array()) {
        let mut out = Vec::new();
        for row in values {
            let arr = row.as_array()?;
            out.push(arr.iter().map(|cell| cell.as_str().unwrap_or("").to_string()).collect());
        }
        return Some(out);
    }

    if let Some(ranges) = result.get("valueRanges").and_then(|v| v.as_array()) {
        let first = ranges.first()?;
        let values = first.get("values").and_then(|v| v.as_array())?;
        let mut out = Vec::new();
        for row in values {
            let arr = row.as_array()?;
            out.push(arr.iter().map(|cell| cell.as_str().unwrap_or("").to_string()).collect());
        }
        return Some(out);
    }

    if let Some(grid) = result.get("data") {
        if let Some(values) = grid.get("values").and_then(|v| v.as_array()) {
            let mut out = Vec::new();
            for row in values {
                let arr = row.as_array()?;
                out.push(arr.iter().map(|cell| cell.as_str().unwrap_or("").to_string()).collect());
            }
            return Some(out);
        }
    }

    None
}

fn resolve_row_number_by_repo_url_and_lote_id(
    spreadsheet_id: &str,
    repo_url: &str,
    lote_id: &str,
) -> io::Result<u32> {
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": "D2:G",
            "include_grid_data": false
        }),
    )?;
    let values = extract_values_2d(&result).unwrap_or_default();
    let needle = repo_url.trim_end_matches('/').to_ascii_lowercase();
    let lote_needle = lote_id.trim();

    for (idx, row) in values.iter().enumerate() {
        let repo_cell = row.get(0).map(|s| s.trim()).unwrap_or("");
        let lote_cell = row.get(3).map(|s| s.trim()).unwrap_or("");
        let repo_hay = repo_cell.trim_end_matches('/').to_ascii_lowercase();
        if !repo_hay.is_empty()
            && repo_hay == needle
            && !lote_cell.is_empty()
            && lote_cell == lote_needle
        {
            return Ok((idx as u32) + 2);
        }
    }

    for (idx, row) in values.iter().enumerate() {
        let repo_cell = row.get(0).map(|s| s.trim()).unwrap_or("");
        let lote_cell = row.get(3).map(|s| s.trim()).unwrap_or("");
        if repo_cell.is_empty() && lote_cell.is_empty() {
            return Ok((idx as u32) + 2);
        }
    }

    Ok(((values.len() as u32) + 2).max(2))
}

fn read_status_atualizacao_e_fase(
    spreadsheet_id: &str,
    row_number_1based: u32,
) -> io::Result<(String, String)> {
    let range = format!("A{row_number_1based}:B{row_number_1based}");
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )?;
    let values = extract_values_2d(&result).unwrap_or_default();
    let row = values.get(0).cloned().unwrap_or_default();
    let status_atualizacao = row.get(0).map(|s| s.trim().to_string()).unwrap_or_default();
    let status_fase = row.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
    Ok((status_atualizacao, status_fase))
}

fn update_status_fase_only(
    spreadsheet_id: &str,
    row_number_1based: u32,
    status_fase: &str,
) -> io::Result<()> {
    let range = format!("B{row_number_1based}:B{row_number_1based}");
    let _ = call_mcp(
        "batch_update_cells",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "ranges": {
                range: [[status_fase]]
            }
        }),
    )?;
    Ok(())
}

fn confirm_sheet_write(row_number_1based: u32, expected_repo_id: &str) -> io::Result<bool> {
    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let expected_pretty = expected_repo_id.replace("/", " / ");
    let range = format!("C{}:C{}", row_number_1based, row_number_1based);
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )?;
    let values = extract_values_2d(&result).unwrap_or_default();
    let cell = values
        .get(0)
        .and_then(|r| r.get(0))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    Ok(cell == expected_repo_id || cell == expected_pretty)
}

fn inspect_row_width_a_to_cf(row_number_1based: u32) -> io::Result<usize> {
    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_ID")
        .map_err(|_| io::Error::other("Missing GOOGLE_SHEETS_ID"))?;
    let range = format!("A{}:CF{}", row_number_1based, row_number_1based);
    let result = call_mcp(
        "get_sheet_data",
        json!({
            "spreadsheet_id": spreadsheet_id,
            "sheet": "MASTER_SOLUTIONS",
            "range": range,
            "include_grid_data": false
        }),
    )?;
    let values = extract_values_2d(&result).unwrap_or_default();
    Ok(values.get(0).map(|r| r.len()).unwrap_or(0))
}

async fn run_phase_binary(binary_stem: &str, repo_id: &str) -> io::Result<u128> {
    use std::process::Stdio;

    let started = Instant::now();
    let mut exe_path = std::env::current_exe()?;
    let bin_name = if cfg!(target_os = "windows") {
        if binary_stem.to_ascii_lowercase().ends_with(".exe") {
            binary_stem.to_string()
        } else {
            format!("{binary_stem}.exe")
        }
    } else {
        binary_stem.to_string()
    };
    exe_path.set_file_name(bin_name);

    let status = tokio::process::Command::new(exe_path)
        .args(["--repo", repo_id])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|e| io::Error::other(format!("Falha ao executar fase '{binary_stem}': {e}")))?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "Fase '{binary_stem}' retornou exit code != 0: {status}"
        )));
    }

    Ok(started.elapsed().as_millis())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let level = match rust_log.to_ascii_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    let started_total = Instant::now();
    let root_dir = workspace_root()?;
    dotenvy::from_path(root_dir.join(".env")).ok();

    let CliArgs { repo_id, e2e_full, .. } = parse_cli_args();
    if e2e_full {
        info!(repo_id = %repo_id, "E2E FULL: iniciando F0 → F4 (disparo completo)");
    } else {
        info!(repo_id = %repo_id, "E2E: iniciando F3 → F4 (munição real)");
    }

    let db_path = root_dir.join(".soda_data").join("soda_heuristic_vault.db");
    let conn = Connection::open(&db_path).map_err(|e| {
        io::Error::other(format!("Falha ao abrir vault em {}: {}", db_path.display(), e))
    })?;

    let (lote_id, repo_url) = fetch_repo_core(&conn, &repo_id).unwrap_or_else(|_| {
        (
            "LOTE_E2E".to_string(),
            format!("https://github.com/{}", repo_id),
        )
    });
    let lote_id = std::env::var("SODA_LOTE_ID_OVERRIDE")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or(lote_id);

    if e2e_full {
        if let Ok(spreadsheet_id) = std::env::var("GOOGLE_SHEETS_ID") {
            let row_number =
                resolve_row_number_by_repo_url_and_lote_id(&spreadsheet_id, &repo_url, &lote_id)?;
            let (status_atualizacao, _status_fase) =
                read_status_atualizacao_e_fase(&spreadsheet_id, row_number)?;
            if status_atualizacao == "PENDENTE_FASE_0" {
                info!(
                    row_number,
                    "Orquestrador: gatilho HITL detectado (PENDENTE_FASE_0). Executando apenas F0"
                );
                let phase0_ms = run_phase_binary("f0_harvester_cli", &repo_id).await?;
                update_status_fase_only(&spreadsheet_id, row_number, "FASE_0_OK")?;
                info!(
                    phase0_ms,
                    row_number,
                    "Orquestrador: F0 concluída; status_fase atualizado; encerrando sem LLM"
                );
                return Ok(());
            }
        }
    }

    let phase1_ms = if e2e_full {
        run_phase_binary("f0_harvester_cli", &repo_id).await?
    } else {
        0
    };
    let phase1_5_ms = if e2e_full {
        run_phase_binary("f1_distiller_cli", &repo_id).await?
    } else {
        0
    };
    let phase2_ms = if e2e_full {
        run_phase_binary("f2_swarm_cli", &repo_id).await?
    } else {
        0
    };

    let (lens_a, lens_b, lens_c) = fetch_debates(&conn, &repo_id)?;
    let phase2_cost_usd = if e2e_full {
        extract_total_cost_usd_from_lens_json(&lens_a)
            + extract_total_cost_usd_from_lens_json(&lens_b)
            + extract_total_cost_usd_from_lens_json(&lens_c)
    } else {
        0.0
    };

    let seed = try_fetch_repo_heuristics_seed(&conn, &repo_id);
    let (seed_repo_version, seed_ultima_versao_online, seed_licenca, seed_stack_base, seed_declared_description) = seed.unwrap_or_else(|| {
        (
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
            "UNKNOWN".to_string(),
        )
    });

    let now = now_epoch_secs()?;
    let (repo_version_from_repositorios, ultima_versao_online_from_repositorios) =
        try_fetch_repositorios_release_info(&conn, &repo_id);
    let mut repo_version = repo_version_from_repositorios
        .or_else(|| {
            let trimmed = seed_repo_version.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        })
        .unwrap_or_else(|| "UNKNOWN".to_string());
    let repo_version_lower = repo_version.to_ascii_lowercase();
    let mut ultima_versao_online = ultima_versao_online_from_repositorios
        .or_else(|| {
            let trimmed = seed_ultima_versao_online.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        })
        .unwrap_or_else(|| "UNKNOWN".to_string());

    if repo_version_lower == "main"
        || repo_version_lower == "master"
        || repo_version_lower == "unknown"
        || ultima_versao_online.to_ascii_lowercase() == "unknown"
    {
        if let Some(tag) = try_fetch_github_latest_release_tag(&repo_url).await {
            repo_version = tag.clone();
            ultima_versao_online = tag;
        } else if let Ok(url) = Url::parse(&repo_url) {
            let limiter = RateLimiter;
            if let Ok(meta) = CommunityMetaFetcher::fetch(&url, &limiter).await {
                if let Some(sha) = meta.last_commit_sha {
                    let short = sha.chars().take(7).collect::<String>();
                    if !short.is_empty() {
                        repo_version = short.clone();
                        ultima_versao_online = short;
                    }
                }
            }
        }
    }

    if let Some(tag) = try_fetch_github_latest_release_tag(&repo_url).await {
        repo_version = tag.clone();
        ultima_versao_online = tag;
    }
    info!(repo_version = %repo_version, "E2E: repo_version resolvido");

    let mut licenca = seed_licenca.trim().to_string();
    let mut stack_base = seed_stack_base.trim().to_string();
    let mut declared_description = seed_declared_description.trim().to_string();

    if is_unknown_like(&stack_base) {
        if let Some(text) = fetch_raw_artifact_text(&conn, &repo_id, "blob_02_dependency_manifest") {
            if let Some(derived) = derive_stack_base_from_manifest_blob(&text) {
                stack_base = derived;
            }
        }
    }

    if is_unknown_like(&licenca) {
        if let Some(text) = fetch_raw_artifact_text(&conn, &repo_id, "blob_09_community_meta") {
            if let Some(derived) = derive_license_from_community_meta_json(&text) {
                licenca = derived;
            }
        }
        if is_unknown_like(&licenca) {
            if let Some(text) = fetch_raw_artifact_text(&conn, &repo_id, "blob_01_promessa_readme") {
                if let Some(derived) = derive_license_from_readme(&text) {
                    licenca = derived;
                }
            }
        }
    }

    if is_unknown_like(&declared_description) {
        if let Some(text) = fetch_raw_artifact_text(&conn, &repo_id, "blob_01_promessa_readme") {
            if let Some(derived) = derive_declared_description_from_readme(&text) {
                declared_description = derived;
            }
        }
    }

    info!(
        repo_id = %repo_id,
        licenca = %licenca,
        stack_base = %stack_base,
        declared_description = %declared_description,
        "E2E: sementes do Bloco 0 resolvidas a partir do vault/blobs"
    );
    let block0 = Block0Context {
        status_atualizacao: "EM_PROCESSAMENTO".to_string(),
        status_fase: "F3".to_string(),
        project_name: repo_id.clone(),
        repo_url,
        repo_version,
        ultima_versao_online,
        lote_id: lote_id.clone(),
        data_ultima_analise: now,
        analise_origem: "SODA_E2E_F3".to_string(),
        licenca,
        stack_base,
        declared_description,
        lente_a_sentido_prod_ux: lens_a,
        lente_b_estrutura_arq: lens_b,
        lente_c_realidade_ops: lens_c,
    };

    let formatter = OpenRouterFormatterClient::from_env().map_err(io::Error::other)?;
    let formatter_model = std::env::var("OPENROUTER_FORMATTER_MODEL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| OFFICIAL_FORMATTER_MODEL.to_string());
    let cfg = Phase3Config {
        model: formatter_model.clone(),
        max_attempts_per_block: 3,
    };

    let started_phase3_4 = Instant::now();
    let phase3_out = match run_phase3_sgr(&formatter, &cfg, block0).await {
        Ok(out) => out,
        Err(Phase3Error::RetryExhausted { block, attempts, message }) => {
            error!(block, attempts, message = %message, "E2E: falha terminal no SGR após retries");
            return Err(io::Error::other("Falha terminal no SGR"));
        }
        Err(e) => {
            error!(error = %e, "E2E: falha no SGR");
            return Err(io::Error::other(format!("Falha SGR: {}", e)));
        }
    };

    info!("E2E: F3 concluída. Iniciando F4 (carga atômica Sheets)");
    let block3_justifications = phase3_out.block3_justifications;
    let row = phase3_out.row;
    let row_number = SsotInjector::inject_ssot(&repo_id, row, block3_justifications, now)
        .await
        .map_err(|e| io::Error::other(format!("Falha na F4 (Carga SSOT Sheets): {}", e)))?;

    let confirmed = confirm_sheet_write(row_number, &repo_id)?;
    if !confirmed {
        return Err(io::Error::other(
            "E2E: atualização enviada, mas confirmação via leitura não bateu",
        ));
    }

    let width_a_to_cf = inspect_row_width_a_to_cf(row_number)?;
    info!(width_a_to_cf, "E2E: inspeção pós-write (A:CF) para largura do row");

    let usage = formatter.usage_totals();
    let elapsed_phase3_4_ms = started_phase3_4.elapsed().as_millis();
    info!(
        elapsed_ms = elapsed_phase3_4_ms,
        prompt_tokens = usage.prompt_tokens,
        completion_tokens = usage.completion_tokens,
        total_tokens = usage.total_tokens,
        total_cost_usd = usage.total_cost_usd,
        "E2E: concluído com confirmação de escrita no Sheets"
    );

    let e2e_full_total_cost_usd = phase2_cost_usd + usage.total_cost_usd;
    let e2e_full_total_ms =
        phase1_ms + phase1_5_ms + phase2_ms + (elapsed_phase3_4_ms as u128);
    let report_path = etl_report_path(&root_dir, &repo_id)?;
    let mut report = String::new();
    report.push_str(&format!(
        "\n\n=== FASE 3-4: SGR + SSOT @ {} ===\n\n",
        now_brt_rfc3339()
    ));
    report.push_str(&format!("repo_id={}\n", repo_id));
    report.push_str(&format!("row_number={}\n", row_number));
    report.push_str(&format!("model_used={}\n", formatter_model));
    report.push_str(&format!("lote_id={}\n", lote_id));
    report.push_str(&format!("latency_f3_f4_ms={}\n", elapsed_phase3_4_ms));
    report.push_str(&format!(
        "latency_total_ms={}\n",
        if e2e_full { e2e_full_total_ms } else { elapsed_phase3_4_ms as u128 }
    ));
    report.push_str(&format!("prompt_tokens={}\n", usage.prompt_tokens));
    report.push_str(&format!("completion_tokens={}\n", usage.completion_tokens));
    report.push_str(&format!("total_tokens={}\n", usage.total_tokens));
    report.push_str(&format!(
        "total_cost_usd={:.6}\n",
        if e2e_full { e2e_full_total_cost_usd } else { usage.total_cost_usd }
    ));
    report.push_str(&format!("cost_f2_usd={:.6}\n", phase2_cost_usd));
    report.push_str(&format!("cost_f3_f4_usd={:.6}\n", usage.total_cost_usd));
    report.push_str(&format!("sheets_write_confirmed={}\n", confirmed));
    report.push_str(&format!("row_width_a_to_cf={}\n", width_a_to_cf));
    report.push_str(&format!(
        "elapsed_total_wall_ms={}\n",
        started_total.elapsed().as_millis()
    ));

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&report_path)
        .map_err(|e| io::Error::other(format!("Falha ao abrir relatório ETL {}: {}", report_path.display(), e)))?;
    file.write_all(report.as_bytes())
        .map_err(|e| io::Error::other(format!("Falha ao anexar relatório ETL: {}", e)))?;
    info!(report = %report_path.display(), "E2E: relatório ETL anexado");
    Ok(())
}
