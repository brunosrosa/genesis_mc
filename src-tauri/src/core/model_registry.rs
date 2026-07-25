use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use memmap2::MmapOptions;
use rusqlite::{params, Connection};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub file_path: String,
    pub model_name: String,
    pub family: String,
    pub parameters: String,
    pub context_length: u64,
    pub quantization: String,
    pub capabilities: Vec<String>,
    pub file_size_bytes: u64,
    pub is_active: bool,
    pub tier1_passed: bool,
    pub success_rate_ema: f64,
    pub ema_latency_ms: f64,
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
            0 | 1 | 7 => self.read_slice(1).is_some(),
            2 | 3 => self.read_slice(2).is_some(),
            4 | 5 | 6 => self.read_slice(4).is_some(),
            10 | 11 | 12 => self.read_slice(8).is_some(),
            8 => self.read_string().is_some(),
            9 => {
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

/// Extrai metadados do cabeçalho GGUF em O(1) e Zero-Copy via memmap2 (sem carregar tensores em RAM).
pub fn parse_gguf_metadata_zero_copy(file_path: &Path) -> Option<ModelMetadata> {
    let f = File::open(file_path).ok()?;
    let meta = f.metadata().ok()?;
    let file_size = meta.len();
    if file_size < 24 {
        return None;
    }

    let mmap = unsafe { MmapOptions::new().map(&f).ok()? };
    if mmap.len() < 24 || &mmap[..4] != b"GGUF" {
        return None;
    }

    let mut cursor = ByteCursor::new(&mmap);
    cursor.pos = 4; // Skip magic

    let _version = cursor.read_u32()?;
    let _tensor_count = cursor.read_u64()?;
    let kv_count = cursor.read_u64()?;

    let mut family = String::new();
    let mut name = String::new();
    let mut chat_template = String::new();
    let mut context_length: u64 = 4096;
    let mut file_type_enum: Option<u32> = None;

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

    let model_name = if !name.trim().is_empty() {
        name.trim().to_string()
    } else {
        file_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "UnknownModel".to_string())
    };

    if family.is_empty() {
        family = infer_family(&filename, &model_name);
    }
    let parameters = infer_params(&parent_dir, &filename, &model_name);
    let quantization = file_type_enum
        .map(file_type_to_quant_str)
        .unwrap_or_else(|| "GGUF".to_string());

    let capabilities = infer_capabilities(&family, &model_name, &filename, &chat_template);

    Some(ModelMetadata {
        file_path: file_path.to_string_lossy().to_string(),
        model_name,
        family,
        parameters,
        context_length,
        quantization,
        capabilities,
        file_size_bytes: file_size,
        is_active: true,
        tier1_passed: false,
        success_rate_ema: 0.0,
        ema_latency_ms: 0.0,
    })
}

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
        28 => "BF16",
        _ => "GGUF_CUSTOM",
    }
    .to_string()
}

fn infer_family(filename: &str, name: &str) -> String {
    let combined = format!("{} {}", filename, name).to_lowercase();
    if combined.contains("qwen3.5") || combined.contains("qwen-3.5") {
        "Qwen3.5".to_string()
    } else if combined.contains("qwen") {
        "Qwen".to_string()
    } else if combined.contains("llama") {
        "Llama".to_string()
    } else if combined.contains("granite") {
        "Granite".to_string()
    } else if combined.contains("phi") {
        "Phi".to_string()
    } else {
        "Generic".to_string()
    }
}

fn infer_params(parent_dir: &str, filename: &str, name: &str) -> String {
    let combined = format!("{} {} {}", parent_dir, filename, name);
    let re_b = regex::Regex::new(r"(?i)\b(\d+(?:\.\d+)?)\s*[bB]\b").ok();
    if let Some(re) = re_b {
        if let Some(cap) = re.captures(&combined) {
            return format!("{}B", &cap[1]);
        }
    }
    "Unknown".to_string()
}

fn infer_capabilities(arch: &str, model_name: &str, filename: &str, chat_template: &str) -> Vec<String> {
    let mut caps = Vec::new();
    let combined = format!("{} {} {}", arch, model_name, filename).to_lowercase();
    let tpl_lower = chat_template.to_lowercase();

    if tpl_lower.contains("tools") || tpl_lower.contains("tool_call") || combined.contains("coder") {
        caps.push("TOOL_CALLING".to_string());
    }
    if caps.is_empty() {
        caps.push("BASE".to_string());
    }
    caps
}

pub fn resolve_db_path() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = if cwd.ends_with("src-tauri") {
        cwd.parent().unwrap_or(&cwd).to_path_buf()
    } else {
        cwd
    };
    root.join(".soda_data").join("soda_heuristic_vault.db")
}

pub fn init_model_registry_db(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let conn = Connection::open(db_path)
        .map_err(|e| format!("Falha ao abrir soda_heuristic_vault.db: {e}"))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS model_registry (
            file_path TEXT PRIMARY KEY,
            model_name TEXT NOT NULL DEFAULT '',
            family TEXT NOT NULL DEFAULT '',
            parameters TEXT NOT NULL DEFAULT '',
            context_length INTEGER NOT NULL DEFAULT 4096,
            quantization TEXT NOT NULL DEFAULT '',
            capabilities TEXT NOT NULL DEFAULT '[]',
            file_size_bytes INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            tier1_passed INTEGER NOT NULL DEFAULT 0,
            success_rate_ema REAL NOT NULL DEFAULT 0.0,
            ema_latency_ms REAL NOT NULL DEFAULT 0.0,
            last_seen TEXT NOT NULL DEFAULT (DATETIME('now'))
        );",
        [],
    )
    .map_err(|e| format!("Falha ao criar tabela model_registry: {e}"))?;

    Ok(conn)
}

/// Sincroniza os modelos locais para a tabela `model_registry`.
/// Regra de Ouro (Garbage Collection): Modelos que sumiram do disco sofrem SOFT DELETION (`is_active = 0`). DELETE físico é proibido!
pub fn sync_local_models_to_registry(conn: &Connection, models_dir: &Path) -> Result<usize, String> {
    let mut scanned_paths = Vec::new();

    for entry in WalkDir::new(models_dir).into_iter().flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "gguf" {
                    let filename = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default();

                    if !filename.contains("mmproj") {
                        if let Some(m) = parse_gguf_metadata_zero_copy(path) {
                            scanned_paths.push(m.file_path.clone());
                            let caps_json = serde_json::to_string(&m.capabilities).unwrap_or_else(|_| "[]".to_string());

                            let res = conn.execute(
                                "INSERT INTO model_registry (file_path, model_name, family, parameters, context_length, quantization, capabilities, file_size_bytes, is_active, last_seen)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, DATETIME('now'))
                                 ON CONFLICT(file_path) DO UPDATE SET
                                    model_name=excluded.model_name,
                                    family=excluded.family,
                                    parameters=excluded.parameters,
                                    context_length=excluded.context_length,
                                    quantization=excluded.quantization,
                                    capabilities=excluded.capabilities,
                                    file_size_bytes=excluded.file_size_bytes,
                                    is_active=1,
                                    last_seen=DATETIME('now');",
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
                            );

                            if let Err(e) = res {
                                tracing::error!("Falha ao registrar modelo no SQLite: {e}");
                            }
                        }
                    }
                }
            }
        }
    }

    // SOFT DELETION: Marca modelos ausentes do disco como is_active = 0 sem DELETE físico
    let mut stmt = conn
        .prepare("SELECT file_path FROM model_registry WHERE is_active = 1")
        .map_err(|e| format!("Falha ao preparar consulta SQL: {e}"))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Falha ao iterar sobre registros ativos: {e}"))?;

    let mut missing_paths = Vec::new();
    for row in rows.flatten() {
        if !Path::new(&row).exists() {
            missing_paths.push(row);
        }
    }

    for missing in &missing_paths {
        let _ = conn.execute(
            "UPDATE model_registry SET is_active = 0, last_seen = DATETIME('now') WHERE file_path = ?1",
            params![missing],
        );
        tracing::warn!("Modelo ausente do disco marcado como inativo (is_active = 0): {}", missing);
    }

    Ok(scanned_paths.len())
}

/// Atualiza o resultado da avaliação Tier 1 diretamente no banco SQLite SSOT.
pub fn update_tier1_result(
    conn: &Connection,
    file_path: &str,
    success_rate: f64,
    avg_latency_ms: f64,
    passed: bool,
) -> Result<(), String> {
    let tier1_passed_val = if passed { 1 } else { 0 };

    conn.execute(
        "UPDATE model_registry 
         SET tier1_passed = ?1, success_rate_ema = ?2, ema_latency_ms = ?3, last_seen = DATETIME('now')
         WHERE file_path = ?4",
        params![tier1_passed_val, success_rate, avg_latency_ms, file_path],
    )
    .map_err(|e| format!("Falha ao atualizar resultado Tier 1 no SQLite: {e}"))?;

    Ok(())
}

/// Consulta os modelos locais que foram APROVADOS no Tier 1 e estão ATIVOS em disco.
pub fn fetch_approved_tier1_models(conn: &Connection) -> Result<Vec<PathBuf>, String> {
    let mut stmt = conn
        .prepare("SELECT file_path FROM model_registry WHERE tier1_passed = 1 AND is_active = 1")
        .map_err(|e| format!("Falha ao preparar consulta de modelos aprovados: {e}"))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Falha ao mapear modelos aprovados: {e}"))?;

    let mut paths = Vec::new();
    for r in rows.flatten() {
        paths.push(PathBuf::from(r));
    }

    Ok(paths)
}

/// Coleta modelos locais RECURSIVAMENTE via WalkDir ignorando projetores multimodais (mmproj).
pub fn collect_local_models(models_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(models_dir).into_iter().flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == "gguf" {
                    let filename = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    if !filename.contains("mmproj") {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
    }
    files
}

/// Carrega modelos aprovados a partir do relatório fallback de texto Tier 1.
pub fn load_approved_tier1_models(report_path: &Path) -> Vec<PathBuf> {
    let mut approved = Vec::new();
    let file = match File::open(report_path) {
        Ok(f) => f,
        Err(_) => return approved,
    };

    use std::io::BufRead;
    let reader = std::io::BufReader::new(file);
    let mut current_path: Option<PathBuf> = None;

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();
        if trimmed.starts_with("Path:") {
            let path_str = trimmed.trim_start_matches("Path:").trim();
            current_path = Some(PathBuf::from(path_str));
        } else if trimmed.starts_with("Status:") && trimmed.contains("[APROVADO PARA TIER 2]") {
            if let Some(p) = current_path.take() {
                approved.push(p);
            }
        }
    }

    approved
}
