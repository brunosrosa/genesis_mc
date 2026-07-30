// SODA-CANIBALIZED: Deduplicação de blocos de 5 linhas consecutivas.
// Identifica sequências repetidas de 5 linhas consecutivas e substitui
// as ocorrências duplicadas subsequentes por `// [dedup: 5 lines hidden]`.

pub fn deduplicate_blocks(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 5 {
        return text.to_string();
    }

    let mut result = Vec::new();
    let mut seen_blocks = std::collections::HashSet::new();
    let mut i = 0;

    while i < lines.len() {
        if i + 5 <= lines.len() {
            let block: Vec<&str> = lines[i..i + 5].to_vec();
            let block_key = block.join("\n");

            if seen_blocks.contains(&block_key) {
                result.push("// [dedup: 5 lines hidden]");
                i += 5;
                continue;
            } else {
                seen_blocks.insert(block_key);
            }
        }
        result.push(lines[i]);
        i += 1;
    }

    let mut out = result.join("\n");
    if text.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplicate_blocks_5_lines() {
        let block = "line1\nline2\nline3\nline4\nline5\n";
        let input = format!("{block}extra_line\n{block}");
        let output = deduplicate_blocks(&input);

        assert!(output.contains("// [dedup: 5 lines hidden]"));
        assert_eq!(output.matches("// [dedup: 5 lines hidden]").count(), 1);
    }
}
