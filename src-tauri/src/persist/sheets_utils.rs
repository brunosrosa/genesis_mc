use serde_json::Value;

pub fn normalize_header_cell(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

pub fn col_idx_to_a1(col_idx0: usize) -> String {
    let mut n = col_idx0 + 1;
    let mut out = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        out.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    out
}

pub fn find_col_idx(header_row: &[String], needle: &str) -> Option<usize> {
    let n = normalize_header_cell(needle);
    for (idx, raw) in header_row.iter().enumerate() {
        if normalize_header_cell(raw) == n {
            return Some(idx);
        }
    }
    None
}

pub fn extract_values_2d_strict(value: &Value) -> Result<Vec<Vec<String>>, String> {
    if let Some(err) = value.get("error") {
        let code = err.get("code").and_then(|v| v.as_i64());
        let message = err.get("message").and_then(|v| v.as_str());
        return Err(match (code, message) {
            (Some(c), Some(m)) => format!("Google Sheets API error: code={c} message={m}"),
            (Some(c), None) => format!("Google Sheets API error: code={c}"),
            (None, Some(m)) => format!("Google Sheets API error: message={m}"),
            _ => format!("Google Sheets API error: {err}"),
        });
    }

    let values = if let Some(arr) = value.get("values").and_then(|v| v.as_array()) {
        arr
    } else if let Some(ranges) = value.get("valueRanges").and_then(|v| v.as_array()) {
        let first = ranges
            .first()
            .ok_or_else(|| "Sheets payload inválido: valueRanges vazio".to_string())?;
        first
            .get("values")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Sheets payload inválido: valueRanges[0].values ausente".to_string())?
    } else if let Some(grid) = value.get("data") {
        grid.get("values")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Sheets payload inválido: data.values ausente".to_string())?
    } else {
        return Err("Sheets payload inválido: sem 'values', 'valueRanges' ou 'data.values'".to_string());
    };

    let mut out = Vec::new();
    for row in values {
        let row_arr = row
            .as_array()
            .ok_or_else(|| "Sheets payload inválido: linha não é array".to_string())?;
        out.push(
            row_arr
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect(),
        );
    }
    Ok(out)
}

