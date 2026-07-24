use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use memmap2::MmapOptions;
use rusqlite::{params, Connection};
use serde_json::Value;

#[derive(Debug, Clone)]
struct ModelMetadata {
    file_path: String,
    model_name: String,
    family: String,
    parameters: String,
    context_length: u64,
    quantization: String,
    capabilities: Vec<String>,
    file_size_bytes: u64,
}

struct ByteCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn has_remaining(&self, count: usize) -> bool {
        self.pos.checked_add(count).map_or(false, |end| end <= self.data.len())
    }

    fn read_u32(&mut self) -> Option<u32> {
        if !self.has_remaining(4) {
            return None;
        }
        let bytes = &self.data[self.pos..self.pos + 4];
        self.pos += 4;
        let arr: [u8; 4] = bytes.try_into().ok()?;
        Some(u32::from_le_bytes(arr))
    }

    fn read_u64(&mut self) -> Option<u64> {
        if !self.has_remaining(8) {
            return None;
        }
        let bytes = &self.data[self.pos..self.pos + 8];
        self.pos += 8;
        let arr: [u8; 8] = bytes.try_into().ok()?;
        Some(u64::from_le_bytes(arr))
    }

    fn read_slice(&mut self, len: usize) -> Option<&'a [u8]> {
        if !self.has_remaining(len) {
            return None;
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Some(slice)
    }

    fn read_string(&mut self) -> Option<String> {
        let len = self.read_u64()? as usize;
        let bytes = self.read_slice(len)?;
        Some(String::from_utf8_lossy(bytes).to_string())
    }

    fn skip_value(&mut self, val_type: u32) -> bool {
        match val_type {
            0 | 1 | 7 => self.read_slice(1).is_some(), // UINT8, INT8, BOOL
            2 | 3 => self.read_slice(2).is_some(),     // UINT16, INT16
            4 | 5 | 6 => self.read_slice(4).is_some(), // UINT32, INT32, FLOAT32
            10 | 11 | 12 => self.read_slice(8).is_some(), // UINT64, INT64, FLOAT64
            8 => self.read_string().is_some(),        // STRING
            9 => {                                    // ARRAY
                let elem_type = match self.read_u32() {
                    Some(t) => t,
                    None => return false,
                };
                let arr_len = match self.read_u64() {
                    Some(l) => l,
                    None => return false,
                };
                for _ in 0..arr_len {
                    if !self.skip_value(elem_type) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}

fn is_toxic_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_lowercase();
    if lower.contains("unsloth_gguf")
        || lower.contains("unsloth gguf")
        || lower.contains("lmstudio community")
        || lower.contains("lmstudio_community")
        || lower.contains("unsloth_gguf_")
    {
        return true;
    }
    // Rejeita hashes hexadecimais (comprimento >= 32 e estritamente caracteres hex)
    if trimmed.len() >= 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }
    false
}

fn resolve_clean_model_name(raw_name: &str, file_path: &Path) -> String {
    if !is_toxic_name(raw_name) {
        return raw_name.trim().to_string();
    }

    if let Some(parent) = file_path.parent() {
        if let Some(parent_name) = parent.file_name() {
            let parent_str = parent_name.to_string_lossy().to_string();
            if !parent_str.is_empty() && parent_str.to_lowercase() != "models" {
                return parent_str;
            }
        }
    }

    file_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown Model".to_string())
}

fn infer_capabilities(
    arch: &str,
    model_name: &str,
    filename: &str,
    chat_template: &str,
    has_mtp: bool,
) -> Vec<String> {
    let mut caps = Vec::new();
    let combined = format!("{} {} {}", arch, model_name, filename).to_lowercase();
    let tpl_lower = chat_template.to_lowercase();

    // 1. VISION / MULTIMODAL
    if arch.to_lowercase() == "clip"
        || combined.contains("mmproj")
        || combined.contains("vision")
        || combined.contains("vlm")
        || combined.contains("smolvlm")
    {
        caps.push("VISION".to_string());
    }

    // 2. THINKING / REASONING
    if tpl_lower.contains("<think>")
        || tpl_lower.contains("enable_thinking")
        || tpl_lower.contains("reasoning_effort")
        || combined.contains("reasoning")
        || combined.contains("thinking")
        || combined.contains("r1")
        || combined.contains("qwq")
    {
        caps.push("THINKING".to_string());
    }

    // 3. TOOL CALLING / AGENTIC
    if tpl_lower.contains("tools")
        || tpl_lower.contains("tool_call")
        || tpl_lower.contains("<tools>")
        || combined.contains("instruct")
        || combined.contains("agent")
        || combined.contains("agentic")
        || combined.contains("tool")
        || combined.contains("super-coder")
        || combined.contains("coder")
        || combined.contains("function")
    {
        caps.push("TOOL_CALLING".to_string());
    }

    // 4. MULTI-TOKEN PREDICTION (MTP)
    if has_mtp || combined.contains("mtp") || combined.contains("nextn") {
        caps.push("MTP".to_string());
    }

    // 5. BASE / PROJECTOR FALLBACK
    if caps.is_empty() {
        if combined.contains("mmproj") {
            caps.push("PROJECTOR".to_string());
        } else {
            caps.push("BASE".to_string());
        }
    }

    caps
}

fn parse_gguf_metadata(mmap: &[u8], file_path: &Path, file_size: u64) -> Option<ModelMetadata> {
    if mmap.len() < 24 || &mmap[..4] != b"GGUF" {
        return None;
    }

    let mut cursor = ByteCursor::new(mmap);
    cursor.pos = 4; // skip magic

    let _version = cursor.read_u32()?;
    let _tensor_count = cursor.read_u64()?;
    let kv_count = cursor.read_u64()?;

    let mut family = String::new();
    let mut name = String::new();
    let mut chat_template = String::new();
    let mut context_length: u64 = 4096;
    let mut file_type_enum: Option<u32> = None;
    let mut has_mtp = false;

    for _ in 0..kv_count {
        let key = match cursor.read_string() {
            Some(k) => k,
            None => break,
        };
        let val_type = match cursor.read_u32() {
            Some(t) => t,
            None => break,
        };

        if key == "general.architecture" && val_type == 8 {
            if let Some(s) = cursor.read_string() {
                family = s;
            }
        } else if key == "general.name" && val_type == 8 {
            if let Some(s) = cursor.read_string() {
                name = s;
            }
        } else if (key == "tokenizer.chat_template" || key == "general.chat_template") && val_type == 8 {
            if let Some(s) = cursor.read_string() {
                chat_template = s;
            }
        } else if key.contains("num_nextn_predict_layers") || key.contains("mtp_depth") {
            has_mtp = true;
            cursor.skip_value(val_type);
        } else if key.ends_with(".context_length") || key == "general.context_length" {
            if val_type == 4 || val_type == 2 {
                if let Some(val) = cursor.read_u32() {
                    context_length = val as u64;
                }
            } else if val_type == 10 {
                if let Some(val) = cursor.read_u64() {
                    context_length = val;
                }
            } else {
                cursor.skip_value(val_type);
            }
        } else if key == "general.file_type" && (val_type == 4 || val_type == 2) {
            if let Some(val) = cursor.read_u32() {
                file_type_enum = Some(val);
            }
        } else {
            if !cursor.skip_value(val_type) {
                break;
            }
        }
    }

    let filename = file_path.file_name()?.to_string_lossy();
    let parent_dir = file_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let model_name = resolve_clean_model_name(&name, file_path);

    if family.is_empty() {
        family = extract_family_from_filename(&filename, &model_name);
    }
    let parameters = extract_param_size(&parent_dir, &filename, &model_name);

    let filename_quant = extract_quant_from_filename(&filename);
    let quantization = if filename_quant != "GGUF" {
        filename_quant
    } else {
        file_type_enum
            .map(file_type_to_quant_str)
            .unwrap_or_else(|| "GGUF".to_string())
    };

    let capabilities = infer_capabilities(&family, &model_name, &filename, &chat_template, has_mtp);

    Some(ModelMetadata {
        file_path: file_path.to_string_lossy().to_string(),
        model_name,
        family,
        parameters,
        context_length,
        quantization,
        capabilities,
        file_size_bytes: file_size,
    })
}

fn parse_safetensors_metadata(mmap: &[u8], file_path: &Path, file_size: u64) -> Option<ModelMetadata> {
    if mmap.len() < 8 {
        return None;
    }
    let header_bytes: [u8; 8] = mmap[..8].try_into().ok()?;
    let header_len = u64::from_le_bytes(header_bytes) as usize;
    if mmap.len() < 8 + header_len {
        return None;
    }

    let json_bytes = &mmap[8..8 + header_len];
    let val: Value = serde_json::from_slice(json_bytes).ok()?;
    let filename = file_path.file_name()?.to_string_lossy();
    let parent_dir = file_path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let model_name = filename.to_string();
    let family = extract_family_from_filename(&filename, "");
    let parameters = extract_param_size(&parent_dir, &filename, "");
    let quantization = "BF16/F16".to_string();

    let mut context_length = 4096;
    if let Some(meta) = val.get("__metadata__") {
        if let Some(ctx) = meta.get("max_position_embeddings") {
            if let Some(c) = ctx.as_u64() {
                context_length = c;
            }
        }
    }

    let capabilities = infer_capabilities(&family, &model_name, &filename, "", false);

    Some(ModelMetadata {
        file_path: file_path.to_string_lossy().to_string(),
        model_name,
        family,
        parameters,
        context_length,
        quantization,
        capabilities,
        file_size_bytes: file_size,
    })
}

/// Mapeamento oficial do enum llama_ftype em llama.cpp (llama.h / ggml.h)
fn file_type_to_quant_str(ft: u32) -> String {
    match ft {
        0 => "F32",
        1 => "F16",
        2 => "Q4_0",
        3 => "Q4_1",
        7 => "Q8_0",
        8 => "Q5_0",
        9 => "Q5_1",
        10 => "Q2_K",
        11 => "Q3_K_S",
        12 => "Q3_K_M",
        13 => "Q3_K_L",
        14 => "Q4_K_S",
        15 => "Q4_K_M",
        16 => "Q5_K_S",
        17 => "Q5_K_M",
        18 => "Q6_K",
        19 => "IQ2_XXS",
        20 => "IQ2_XS",
        21 => "IQ3_XXS",
        22 => "IQ1_S",
        23 => "IQ2_S",
        24 => "IQ3_S",
        25 => "IQ3_M",
        26 => "IQ2_M",
        27 => "IQ1_M",
        28 => "BF16",
        29 => "Q4_0_4_4",
        30 => "Q4_0_4_8",
        31 => "Q4_0_8_8",
        32 => "IQ4_NL",
        33 => "IQ4_XS",
        34 => "IQ3_K",
        _ => "GGUF_CUSTOM",
    }
    .to_string()
}

fn extract_family_from_filename(filename: &str, name: &str) -> String {
    let combined = format!("{} {}", filename, name).to_lowercase();
    if combined.contains("qwen3.5") || combined.contains("qwen-3.5") || combined.contains("qwen_3.5") {
        "Qwen3.5".to_string()
    } else if combined.contains("qwen3") {
        "Qwen3".to_string()
    } else if combined.contains("qwen") {
        "Qwen".to_string()
    } else if combined.contains("llama") {
        "Llama".to_string()
    } else if combined.contains("phi") {
        "Phi".to_string()
    } else if combined.contains("gemma") {
        "Gemma".to_string()
    } else if combined.contains("granite") {
        "Granite".to_string()
    } else if combined.contains("nemotron") {
        "Nemotron".to_string()
    } else if combined.contains("lfm") || combined.contains("liquid") {
        "Liquid/LFM".to_string()
    } else if combined.contains("deepseek") {
        "DeepSeek".to_string()
    } else if combined.contains("starcoder") {
        "StarCoder".to_string()
    } else {
        "Generic".to_string()
    }
}

fn extract_param_size(parent_dir: &str, filename: &str, name: &str) -> String {
    let combined = format!("{} {} {}", parent_dir, filename, name);
    
    let re_b = regex::Regex::new(r"(?i)\b(\d+(?:\.\d+)?)\s*[bB]\b").unwrap();
    if let Some(cap) = re_b.captures(&combined) {
        return format!("{}B", &cap[1]);
    }

    let re_b_suffix = regex::Regex::new(r"(?i)(\d+(?:\.\d+)?)[bB]\b").unwrap();
    if let Some(cap) = re_b_suffix.captures(&combined) {
        return format!("{}B", &cap[1]);
    }

    if combined.to_lowercase().contains("4-mini") || combined.to_lowercase().contains("4_mini") {
        return "4B".to_string();
    }

    let re_m = regex::Regex::new(r"(?i)\b(\d+(?:\.\d+)?)\s*[mM]\b").unwrap();
    if let Some(cap) = re_m.captures(&combined) {
        return format!("{}M", &cap[1]);
    }

    "Unknown".to_string()
}

fn extract_quant_from_filename(filename: &str) -> String {
    let upper = filename.to_uppercase();
    for q in &[
        "IQ3_M", "IQ3_XXS", "IQ2_XXS", "Q4_K_M", "Q4_K_S", "Q4_0", "Q4_1",
        "Q5_K_M", "Q5_K_S", "Q5_0", "Q8_0", "F16", "BF16", "Q3_K_M", "Q2_K"
    ] {
        if upper.contains(q) {
            return q.to_string();
        }
    }
    "GGUF".to_string()
}

fn collect_model_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if dir.is_file() {
        files.push(dir.to_path_buf());
        return files;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_model_files(&path));
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "gguf" || ext_str == "safetensors" {
                    files.push(path);
                }
            }
        }
    }
    files
}

fn resolve_db_path() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = if cwd.ends_with("src-tauri") {
        cwd.parent().unwrap_or(&cwd).to_path_buf()
    } else if cwd.join(".soda_data").exists() {
        cwd
    } else if cwd.parent().map_or(false, |p| p.join(".soda_data").exists()) {
        cwd.parent().unwrap().to_path_buf()
    } else {
        cwd
    };
    root.join(".soda_data").join("soda_heuristic_vault.db")
}

fn init_sqlite_vault(db_path: &Path) -> Connection {
    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let conn = Connection::open(db_path).expect("Falha ao abrir soda_heuristic_vault.db");
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS local_models (
            file_path TEXT PRIMARY KEY,
            model_name TEXT NOT NULL DEFAULT '',
            family TEXT NOT NULL,
            parameters TEXT NOT NULL,
            context_length INTEGER NOT NULL,
            quantization TEXT NOT NULL,
            capabilities TEXT NOT NULL DEFAULT '[]',
            file_size_bytes INTEGER NOT NULL,
            last_seen TEXT NOT NULL
        );",
        [],
    )
    .expect("Falha ao criar tabela local_models");

    let _ = conn.execute("ALTER TABLE local_models ADD COLUMN model_name TEXT NOT NULL DEFAULT '';", []);
    let _ = conn.execute("ALTER TABLE local_models ADD COLUMN capabilities TEXT NOT NULL DEFAULT '[]';", []);

    // Instancia a VIEW FinOps Nativa do Roteador (vw_finops_routing)
    conn.execute_batch(
        "DROP VIEW IF EXISTS vw_finops_routing;
         CREATE VIEW vw_finops_routing AS
         SELECT 
            file_path,
            model_name,
            family,
            parameters,
            context_length,
            quantization,
            capabilities,
            file_size_bytes,
            last_seen
         FROM local_models
         ORDER BY 
            family ASC,
            CASE 
                WHEN capabilities LIKE '%MTP%' THEN 0
                WHEN capabilities LIKE '%THINKING%' THEN 1
                WHEN capabilities LIKE '%TOOL_CALLING%' THEN 2
                ELSE 3
            END ASC,
            CAST(
                REPLACE(
                    REPLACE(UPPER(parameters), 'B', ''),
                    'UNKNOWN', '999999'
                ) AS REAL
            ) ASC,
            file_size_bytes DESC;"
    )
    .expect("Falha ao criar VIEW vw_finops_routing");

    conn
}

fn sync_state_to_vault(conn: &mut Connection, scanned_models: &[ModelMetadata]) {
    let tx = conn.transaction().expect("Falha ao iniciar transacao SQLite");

    for m in scanned_models {
        let caps_json = serde_json::to_string(&m.capabilities).unwrap_or_else(|_| "[]".to_string());
        tx.execute(
            "INSERT INTO local_models (file_path, model_name, family, parameters, context_length, quantization, capabilities, file_size_bytes, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, DATETIME('now'))
             ON CONFLICT(file_path) DO UPDATE SET
                model_name=excluded.model_name,
                family=excluded.family,
                parameters=excluded.parameters,
                context_length=excluded.context_length,
                quantization=excluded.quantization,
                capabilities=excluded.capabilities,
                file_size_bytes=excluded.file_size_bytes,
                last_seen=excluded.last_seen;",
            params![
                m.file_path,
                m.model_name,
                m.family,
                m.parameters,
                m.context_length as i64,
                m.quantization,
                caps_json,
                m.file_size_bytes as i64,
            ],
        )
        .expect("Falha ao realizar UPSERT em local_models");
    }

    // Golden Rule: State Sync - Remove registros do banco cujos arquivos nao existem mais fisicamente no disco
    let mut stmt = tx.prepare("SELECT file_path FROM local_models").unwrap();
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap();

    let mut orphaned = Vec::new();
    for row in rows.flatten() {
        if !Path::new(&row).exists() {
            orphaned.push(row);
        }
    }
    drop(stmt);

    for orphan in &orphaned {
        tx.execute("DELETE FROM local_models WHERE file_path = ?1", params![orphan])
            .expect("Falha ao deletar modelo orfao");
    }

    tx.commit().expect("Falha ao efetuar commit no SQLite Vault");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let default_target = "C:\\Users\\rosas\\.lmstudio\\models".to_string();
    let target_path_str = args.get(1).unwrap_or(&default_target);
    let target = Path::new(target_path_str);

    if !target.exists() {
        eprintln!("ERR: Caminho alvo nao encontrado: {}", target.display());
        std::process::exit(1);
    }

    let files = collect_model_files(target);
    let mut scanned_models = Vec::new();

    for file_path in &files {
        let f = match File::open(file_path) {
            Ok(file) => file,
            Err(_) => continue,
        };
        let meta = match f.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let file_size = meta.len();
        if file_size < 4 {
            continue;
        }

        let mmap = match unsafe { MmapOptions::new().map(&f) } {
            Ok(m) => m,
            Err(_) => continue,
        };

        if let Some(m) = parse_gguf_metadata(&mmap, file_path, file_size) {
            scanned_models.push(m);
        } else if let Some(m) = parse_safetensors_metadata(&mmap, file_path, file_size) {
            scanned_models.push(m);
        }
    }

    let db_path = resolve_db_path();
    let mut conn = init_sqlite_vault(&db_path);
    sync_state_to_vault(&mut conn, &scanned_models);

    println!(
        "SUCCESS: Scanner finalizado com sucesso. Modelos escaneados e sincronizados: {}",
        scanned_models.len()
    );

    // Print top 5 rows from native VIEW vw_finops_routing for verification
    let mut stmt = conn
        .prepare("SELECT file_path, model_name, family, parameters, context_length, quantization, capabilities, file_size_bytes FROM vw_finops_routing LIMIT 5")
        .unwrap();

    let model_rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })
        .unwrap();

    println!("\n=== TOP MODELOS ROTEADOS PELA VIEW FINOPS (vw_finops_routing) ===");
    for (idx, r) in model_rows.flatten().enumerate() {
        println!(
            "[{}] Nome: {:<35} | Família: {:<10} | Params: {:<6} | Contexto: {:<6} | Quant: {:<8} | Caps: {:<25} | Size: {:.2} GB",
            idx + 1,
            r.1,
            r.2,
            r.3,
            r.4,
            r.5,
            r.6,
            (r.7 as f64) / (1024.0 * 1024.0 * 1024.0)
        );
    }

    std::process::exit(0);
}
