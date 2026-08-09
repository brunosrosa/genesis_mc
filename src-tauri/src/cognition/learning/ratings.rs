use rusqlite::{params, Connection, Error as SqliteError};
use std::time::{SystemTime, UNIX_EPOCH};

/// Sigmoid scaling of raw reward value into range (0.0, 1.0)
pub fn sigmoid_reward(reward_value: f64) -> f64 {
    1.0 / (1.0 + (-reward_value).exp())
}

/// Calculates Bradley-Terry / ELO update for competitor against a dynamic baseline difficulty
/// Equations:
/// E_self = 1.0 / (1.0 + 10.0^((R_baseline - R_self) / 400.0))
/// R_new = R_self + K * (S - E_self)
pub fn calculate_bradley_terry_elo(
    r_self: f64,
    r_baseline: f64,
    k: f64,
    reward_value: f64,
) -> (f64, f64) {
    let s = sigmoid_reward(reward_value);
    let e_self = 1.0 / (1.0 + 10.0_f64.powf((r_baseline - r_self) / 400.0));
    let r_new = r_self + k * (s - e_self);
    (r_new, s)
}

/// Smoothes performance history using exponential moving average (EMA)
/// EMA_{t+1} = alpha * S + (1.0 - alpha) * EMA_t
pub fn update_ema(current_ema: f64, s: f64, alpha: f64) -> f64 {
    alpha * s + (1.0 - alpha) * current_ema
}

/// Ensures a rating target entry exists in `weevolve_ratings` table
pub fn ensure_rating_target(conn: &Connection, target_id: &str) -> Result<(), SqliteError> {
    let rating_type = if target_id.starts_with("model:") {
        "MODEL"
    } else if target_id.starts_with("tool:") {
        "TOOL"
    } else if target_id.starts_with("prompt:") {
        "PROMPT"
    } else {
        "MODEL"
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    conn.execute(
        "INSERT OR IGNORE INTO weevolve_ratings (target_id, rating_type, elo_rating, ema_score, total_matches, updated_at)
         VALUES (?, ?, 1200.0, 1.0, 0, ?)",
        params![target_id, rating_type, now],
    )?;
    Ok(())
}

/// Updates rating and EMA score in SQLite database given a new reward signal
pub fn update_rating_in_db(
    conn: &Connection,
    target_id: &str,
    reward_value: f64,
) -> Result<(f64, f64), SqliteError> {
    ensure_rating_target(conn, target_id)?;

    let (r_self, ema_t, total_matches): (f64, f64, i64) = conn.query_row(
        "SELECT elo_rating, ema_score, total_matches FROM weevolve_ratings WHERE target_id = ?",
        params![target_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    let (r_new, s) = calculate_bradley_terry_elo(r_self, 1200.0, 32.0, reward_value);
    let ema_new = update_ema(ema_t, s, 0.15);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    conn.execute(
        "UPDATE weevolve_ratings SET elo_rating = ?, ema_score = ?, total_matches = ?, updated_at = ? WHERE target_id = ?",
        params![r_new, ema_new, total_matches + 1, now, target_id],
    )?;

    eprintln!(
        "[WeEvolve Bradley-Terry ELO] target: {}, reward: {:.2}, S: {:.4}, ELO: {:.2} -> {:.2}, EMA: {:.4} -> {:.4}",
        target_id, reward_value, s, r_self, r_new, ema_t, ema_new
    );

    Ok((r_new, ema_new))
}

/// Reads current ELO rating and EMA score for target_id from SQLite database
pub fn get_rating_from_db(conn: &Connection, target_id: &str) -> Result<(f64, f64), SqliteError> {
    ensure_rating_target(conn, target_id)?;
    conn.query_row(
        "SELECT elo_rating, ema_score FROM weevolve_ratings WHERE target_id = ?",
        params![target_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}
