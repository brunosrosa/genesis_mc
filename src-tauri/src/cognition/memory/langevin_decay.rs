// SOULS V6 — Memory Module: Langevin Decay (Metabolismo Estocástico & Invariância STABLE)
// Conforme ADR-001, ADR-005, ADR-040 e Marco VI.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::time::{SystemTime, UNIX_EPOCH};
use tinyrand::{Rand, Seeded, StdRand};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PoincareVector {
    pub x: f64,
    pub y: f64,
}

impl PoincareVector {
    pub fn new(x: f64, y: f64) -> Self {
        let (px, py) = proj_poincare((x, y));
        Self { x: px, y: py }
    }

    pub fn norm(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

/// Poincaré Boundary Protection:
/// Força que se ||v|| >= 1.0 (ou limite estrito), o vetor seja escalado de volta para 0.9999.
pub fn proj_poincare(v: (f64, f64)) -> (f64, f64) {
    let norm = (v.0 * v.0 + v.1 * v.1).sqrt();
    if norm >= 0.9999 || norm.is_nan() {
        if norm == 0.0 || norm.is_nan() {
            (0.0, 0.0)
        } else {
            let scale = 0.9999 / norm;
            (v.0 * scale, v.1 * scale)
        }
    } else {
        v
    }
}

/// Perturbação estocástica gaussiana 2D via transformação Box-Muller utilizando a crate `tinyrand`.
pub fn box_muller_tinyrand(rng: &mut StdRand) -> (f64, f64) {
    let raw1 = rng.next_u64();
    let raw2 = rng.next_u64();

    let u1 = ((raw1 & 0x001F_FFFF_FFFF_FFFF) as f64 + 1.0) / (0x0020_0000_0000_0000u64 as f64);
    let u2 = ((raw2 & 0x001F_FFFF_FFFF_FFFF) as f64 + 1.0) / (0x0020_0000_0000_0000u64 as f64);

    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * PI * u2;
    (r * theta.cos(), r * theta.sin())
}

/// Calcula a atualização do score de relevância segundo a Equação de Langevin:
/// S_{t+1} = S_t * e^(-lambda * dt) + sigma * sqrt(dt) * eta_t
///
/// Para nós 'STABLE', lambda = 0.0 e sigma = 0.0 (imunes ao esquecimento).
pub fn compute_langevin_score(
    initial_score: f64,
    stability_status: &str,
    lambda: f64,
    sigma: f64,
    delta_t: f64,
    eta_noise: f64,
) -> f64 {
    if stability_status == "STABLE" {
        // Âncora invariante: sem decaimento
        initial_score
    } else {
        let decay_factor = (-lambda * delta_t).exp();
        let stochastic_term = sigma * delta_t.sqrt() * eta_noise;
        (initial_score * decay_factor + stochastic_term).clamp(0.0, 1.0)
    }
}

/// Aplica um ciclo de decaimento de Langevin estocástico para todos os nós 'EVOLVING' e preserva os nós 'STABLE'.
pub fn apply_langevin_decay(
    conn: &Connection,
    lambda_decay: f64,
    sigma_diffusion: f64,
    delta_t: f64,
) -> Result<usize, String> {
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut rng = StdRand::seed(now_epoch as u64);

    let mut stmt = conn
        .prepare("SELECT memory_id, stability_status, relevance_score, poincare_x, poincare_y FROM souls_memory_nodes")
        .map_err(|e| format!("Erro ao preparar SELECT Langevin: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, f64>(4)?,
            ))
        })
        .map_err(|e| format!("Erro ao executar query Langevin: {}", e))?;

    let mut updates = Vec::new();

    for res in rows {
        let (mem_id, status, score, px, py) = match res {
            Ok(tuple) => tuple,
            Err(_) => continue,
        };

        if status == "STABLE" {
            // Âncoras STABLE são imutáveis contra decaimento
            continue;
        }

        let (eta_x, eta_y) = box_muller_tinyrand(&mut rng);

        // Atualização do score de relevância
        let new_score = compute_langevin_score(score, &status, lambda_decay, sigma_diffusion, delta_t, eta_x);

        // Atualização das coordenadas Poincaré
        let eta_pgd = 0.05;
        let noise_scale = (2.0 * sigma_diffusion * delta_t).sqrt();
        let next_x = px - eta_pgd * px + noise_scale * eta_x;
        let next_y = py - eta_pgd * py + noise_scale * eta_y;

        let (final_x, final_y) = proj_poincare((next_x, next_y));
        let norm = (final_x * final_x + final_y * final_y).sqrt();

        let new_status = if new_score <= 0.05 || norm >= 0.95 {
            "SUPERSEDED"
        } else {
            "EVOLVING"
        };

        updates.push((mem_id, new_score, final_x, final_y, new_status));
    }

    drop(stmt);

    let mut updated_count = 0;
    for (mem_id, new_sc, fx, fy, new_st) in updates {
        let sql = "UPDATE souls_memory_nodes SET relevance_score = ?1, poincare_x = ?2, poincare_y = ?3, stability_status = ?4, updated_at = ?5 WHERE memory_id = ?6";
        if let Err(e) = conn.execute(sql, params![new_sc, fx, fy, new_st, now_epoch, mem_id]) {
            eprintln!("[langevin_decay] ALERTA: Falha ao atualizar nó {}: {}", mem_id, e);
        } else {
            updated_count += 1;
        }
    }

    Ok(updated_count)
}
