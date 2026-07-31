use std::fmt::{self, Write as _};
use std::fs::OpenOptions;
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::registry::LookupSpan;

pub fn enable_virtual_terminal() {
    #[cfg(windows)]
    {
        let _ = enable_ansi_support::enable_ansi_support();
    }
}

pub fn parse_log_level_from_env() -> Level {
    match std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

pub fn init_cli_tracing(_level: Level) {
    enable_virtual_terminal();
    let ansi =
        (io::stderr().is_terminal() || io::stdout().is_terminal()) && std::env::var_os("NO_COLOR").is_none();
    let formatter = SoulsEventFormatter::new(ansi, supports_truecolor());

    use tracing_subscriber::EnvFilter;

    // Configura o EnvFilter para priorizar souls_mc e silenciar ruído do globset
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,souls_mc=debug,souls_mc_lib=debug,globset=info"));

    let _ = tracing_subscriber::fmt()
        .with_ansi(ansi)
        .event_format(formatter)
        .with_writer(io::stderr)
        .with_env_filter(filter)
        .try_init();
}

pub fn append_plaintext_report(path: &Path, text: &str) -> io::Result<()> {
    let clean = strip_ansi_codes(text);
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(clean.as_bytes())
}

pub fn strip_ansi_codes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\u{1b}' {
            out.push(ch);
            continue;
        }
        match chars.peek().copied() {
            Some('[') => {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                for next in chars.by_ref() {
                    if next == '\u{7}' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[derive(Clone, Copy)]
struct SoulsEventFormatter {
    ansi: bool,
    truecolor: bool,
}

impl SoulsEventFormatter {
    fn new(ansi: bool, truecolor: bool) -> Self {
        Self { ansi, truecolor }
    }
}

impl<S, N> FormatEvent<S, N> for SoulsEventFormatter
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let mut visitor = SoulsFieldVisitor::default();
        event.record(&mut visitor);
        let kind = classify_event(metadata.level(), visitor.message.as_deref(), &visitor.fields);
        let base = phase_base_color_for_target(metadata.target());
        let label = match kind {
            EventKind::Processing => "[PROC]",
            EventKind::Success => "[OK]",
            EventKind::Warning => "[WARN]",
            EventKind::Error => "[ERR]",
            EventKind::Finops => "[FINOPS]",
        };
        if self.ansi {
            writer.write_str(base)?;
        }
        let label_color = match kind {
            EventKind::Processing => COLOR_PROCESSING,
            EventKind::Warning => COLOR_WARNING,
            EventKind::Error => COLOR_ERROR,
            EventKind::Success => COLOR_SUCCESS,
            _ => base,
        };
        if self.ansi {
            writer.write_str(label_color)?;
        }
        writer.write_str(label)?;
        if self.ansi {
            writer.write_str(SGR_RESET_ALL)?;
            writer.write_str(base)?;
        }
        write!(writer, " ")?;
        let message = visitor
            .message
            .as_deref()
            .unwrap_or_else(|| metadata.target());
        write_with_semantic_highlights(&mut writer, message, base, self.ansi, self.truecolor)?;
        for (key, value) in &visitor.fields {
            write!(writer, " {}=", key)?;
            if is_finops_key(key) {
                if self.ansi {
                    writer.write_str(COLOR_WHITE)?;
                }
                writer.write_str(value)?;
                if self.ansi {
                    writer.write_str(base)?;
                }
            } else {
                write_with_semantic_highlights(&mut writer, value, base, self.ansi, self.truecolor)?;
            }
        }
        if self.ansi {
            writer.write_str(SGR_RESET_ALL)?;
        }
        writeln!(writer)
    }
}

#[derive(Debug, Clone, Default)]
struct SoulsFieldVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl SoulsFieldVisitor {
    fn push(&mut self, key: &str, value: String) {
        let clean = value.trim_matches('"').to_string();
        if key == "message" {
            self.message = Some(clean);
        } else {
            self.fields.push((key.to_string(), clean));
        }
    }
}

impl Visit for SoulsFieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.push(field.name(), format!("{value:?}"));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push(field.name(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push(field.name(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push(field.name(), value.to_string());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.push(field.name(), value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        let mut out = String::new();
        let _ = write!(&mut out, "{value}");
        self.push(field.name(), out);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventKind {
    Processing,
    Success,
    Warning,
    Error,
    Finops,
}

const PHASE_F0: &str = "\x1b[38;2;217;119;6m";
const PHASE_F1_4: &str = "\x1b[38;2;138;43;226m";
const PHASE_F5: &str = "\x1b[38;2;0;191;255m";
const COLOR_PROCESSING: &str = "\x1b[36m";
const COLOR_WARNING: &str = "\x1b[33m";
const COLOR_ERROR: &str = "\x1b[97;41m";
const COLOR_SUCCESS: &str = "\x1b[32m";
const COLOR_CYAN_NEON: &str = "\x1b[38;2;0;255;255m";
const COLOR_WHITE: &str = "\x1b[97m";
const SGR_RESET_ALL: &str = "\x1b[0m";
const SGR_UNDERLINE_ON: &str = "\x1b[4m";
const SGR_UNDERLINE_OFF: &str = "\x1b[24m";

fn supports_truecolor() -> bool {
    std::env::var("COLORTERM")
        .map(|v| v.to_ascii_lowercase().contains("truecolor"))
        .unwrap_or(false)
        || std::env::var("WT_SESSION").is_ok()
}

fn is_finops_key(key: &str) -> bool {
    matches!(
        key,
        "tokens"
            | "prompt_tokens"
            | "completion_tokens"
            | "total_tokens"
            | "cost"
            | "cost_usd"
            | "total_cost_usd"
            | "elapsed_ms"
            | "duration_ms"
            | "latency_ms"
            | "ms"
            | "total_s"
            | "block_s"
    ) || key.ends_with("_ms")
        || key.ends_with("_usd")
        || key.ends_with("_tokens")
}

fn phase_base_color_for_target(target: &str) -> &'static str {
    if target.contains("f0_harvester_cli") {
        return PHASE_F0;
    }
    if target.contains("f5_deep_formatter_cli") {
        return PHASE_F5;
    }
    if target.contains("f1_distiller_cli")
        || target.contains("f2_swarm_cli")
        || target.contains("f3_synthesizer_cli")
        || target.contains("f_minus_")
        || target.contains("n0_daemon_watcher")
    {
        return PHASE_F1_4;
    }
    PHASE_F1_4
}

fn write_with_semantic_highlights<W: fmt::Write>(
    mut writer: W,
    text: &str,
    base: &str,
    ansi: bool,
    _truecolor: bool,
) -> fmt::Result {
    if !ansi {
        return writer.write_str(text);
    }
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let ch = text[i..].chars().next().unwrap();
        if ch.is_whitespace() {
            writer.write_char(ch)?;
            i += ch.len_utf8();
            continue;
        }
        let start = i;
        while i < bytes.len() {
            let c = text[i..].chars().next().unwrap();
            if c.is_whitespace() {
                break;
            }
            i += c.len_utf8();
        }
        let token = &text[start..i];
        if let Some((core, suffix)) = split_trailing_punct(token) {
            if looks_like_url(core) {
                writer.write_str(SGR_UNDERLINE_ON)?;
                writer.write_str(COLOR_CYAN_NEON)?;
                writer.write_str(core)?;
                writer.write_str(SGR_UNDERLINE_OFF)?;
                writer.write_str(base)?;
                writer.write_str(suffix)?;
                continue;
            }
            if looks_like_owner_repo(core) {
                writer.write_str(COLOR_WHITE)?;
                writer.write_str(core)?;
                writer.write_str(base)?;
                writer.write_str(suffix)?;
                continue;
            }
        }
        writer.write_str(token)?;
    }
    Ok(())
}

fn split_trailing_punct(token: &str) -> Option<(&str, &str)> {
    if token.is_empty() {
        return None;
    }
    let mut end = token.len();
    for (idx, ch) in token.char_indices().rev() {
        if matches!(ch, '.' | ',' | ';' | ':' | ')' | ']' | '}' | '!' | '?') {
            end = idx;
            continue;
        }
        break;
    }
    if end == token.len() {
        return Some((token, ""));
    }
    Some((&token[..end], &token[end..]))
}

fn looks_like_url(token: &str) -> bool {
    token.starts_with("http://") || token.starts_with("https://")
}

fn looks_like_owner_repo(token: &str) -> bool {
    let token = token.trim_matches('/');
    let mut parts = token.split('/');
    let a = match parts.next() {
        Some(v) if !v.is_empty() => v,
        _ => return false,
    };
    let b = match parts.next() {
        Some(v) if !v.is_empty() => v,
        _ => return false,
    };
    if parts.next().is_some() {
        return false;
    }
    fn ok(s: &str) -> bool {
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' ))
    }
    ok(a) && ok(b)
}

fn classify_event(level: &Level, message: Option<&str>, fields: &[(String, String)]) -> EventKind {
    if let Some(semantic_outcome) = fields
        .iter()
        .find_map(|(key, value)| (key == "semantic_outcome").then_some(value.as_str()))
    {
        return match semantic_outcome {
            "ok" => EventKind::Success,
            "informational_non_zero" => EventKind::Warning,
            "lethal_non_zero" => EventKind::Error,
            _ => EventKind::Processing,
        };
    }
    if message == Some("Sandbox: processo efemero concluido") {
        if let Some(exit_code) = fields
            .iter()
            .find_map(|(key, value)| (key == "exit_code").then_some(value))
        {
            return if exit_code == "0" {
                EventKind::Success
            } else {
                EventKind::Warning
            };
        }
    }
    let mut haystack = String::new();
    if let Some(msg) = message {
        haystack.push_str(msg);
        haystack.push(' ');
    }
    for (key, value) in fields {
        if key != "timeout_secs"
            && key != "timeout_ms"
            && key != "args"
            && key != "binary"
            && key != "cmd"
            && key != "command"
            && key != "program"
        {
            haystack.push_str(key);
            haystack.push('=');
            haystack.push_str(value);
            haystack.push(' ');
        }
    }
    let haystack = haystack.to_ascii_lowercase();
    let contains_any = |needles: &[&str]| needles.iter().any(|n| haystack.contains(n));
    if *level == Level::ERROR {
        return EventKind::Error;
    }
    if *level == Level::WARN {
        return EventKind::Warning;
    }
    if fields.iter().any(|(key, _)| is_finops_key(key))
        || contains_any(&["token", "tokens", "custo", "cost", "usd", "latency", "tempo", "elapsed_ms"])
    {
        return EventKind::Finops;
    }
    if *level == Level::DEBUG || *level == Level::TRACE {
        return EventKind::Processing;
    }
    if contains_any(&["erro", "error", "timeout", "panic", "429", "rate limit", "falha", "failed"]) {
        return EventKind::Error;
    }
    if contains_any(&[
        "sucesso",
        "success",
        "concluido",
        "concluida",
        "persistido",
        "persistida",
        "salvo",
        "salva",
        "updated",
        "atualizado",
        "http 200",
        "ok",
    ]) {
        return EventKind::Success;
    }
    EventKind::Processing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_removes_sgr_sequences() {
        let raw = "\x1b[38;2;138;43;226mPROC\x1b[24m\x1b[39m pronto";
        assert_eq!(strip_ansi_codes(raw), "PROC pronto");
    }

    #[test]
    fn sandbox_completion_is_success_only_for_exit_code_zero() {
        let ok = classify_event(
            &Level::INFO,
            Some("Sandbox: processo efemero concluido"),
            &[
                ("exit_code".to_string(), "0".to_string()),
                ("semantic_outcome".to_string(), "ok".to_string()),
            ],
        );
        let err = classify_event(
            &Level::WARN,
            Some("Sandbox: processo efemero concluido"),
            &[
                ("exit_code".to_string(), "7".to_string()),
                ("stderr_bytes".to_string(), "999".to_string()),
                (
                    "semantic_outcome".to_string(),
                    "informational_non_zero".to_string(),
                ),
            ],
        );
        assert_eq!(ok, EventKind::Success);
        assert_eq!(err, EventKind::Warning);
    }

    #[test]
    fn warn_level_with_fail_soft_text_remains_warning() {
        let kind = classify_event(
            &Level::WARN,
            Some("SAST monorepo: normalizacao falhou; descartando payload bruto"),
            &[("scope".to_string(), ".::files-01".to_string())],
        );
        assert_eq!(kind, EventKind::Warning);
    }

    #[test]
    fn semantic_outcome_informational_non_zero_forces_warning() {
        let kind = classify_event(
            &Level::ERROR,
            Some("Sandbox: processo efemero concluido"),
            &[(
                "semantic_outcome".to_string(),
                "informational_non_zero".to_string(),
            )],
        );
        assert_eq!(kind, EventKind::Warning);
    }
}

#[inline]
pub fn dynamic_wyrand() -> tinyrand::Wyrand {
    use tinyrand::Seeded;
    let dummy = 0u8;
    let stack_addr = (&dummy as *const u8 as usize) as u64;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    tinyrand::Wyrand::seed(nanos ^ stack_addr.rotate_left(13))
}

// --- Bare-Metal O(1) Time Utilities (SOULS Group 9 - Zero-Chrono) ---

#[inline]
pub fn now_epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[inline]
pub fn now_epoch_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn epoch_secs_to_date_time(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let secs_u64 = if secs < 0 { 0 } else { secs as u64 };
    let days = secs_u64 / 86400;
    let rem = secs_u64 % 86400;
    let hour = (rem / 3600) as u32;
    let min = ((rem % 3600) / 60) as u32;
    let sec = (rem % 60) as u32;

    let z = (days as i64) + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (if m <= 2 { y + 1 } else { y }) as i32;

    (y, m, d, hour, min, sec)
}

pub fn format_utc_rfc3339(epoch_secs: i64) -> String {
    let (y, m, d, h, min, s) = epoch_secs_to_date_time(epoch_secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, h, min, s)
}

pub fn format_brt_rfc3339(epoch_secs: i64) -> String {
    let brt_secs = epoch_secs - 3 * 3600;
    let (y, m, d, h, min, s) = epoch_secs_to_date_time(brt_secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}-03:00", y, m, d, h, min, s)
}

pub fn now_utc_rfc3339() -> String {
    format_utc_rfc3339(now_epoch_secs())
}

pub fn now_brt_rfc3339() -> String {
    format_brt_rfc3339(now_epoch_secs())
}

