// SOULS V6 — Memory Module: Langevin Decay in Poincaré Disk (ADR-040)
// Algoritmo de decaimento estocástico PGD com Poincaré Boundary Protection e Box-Muller Gaussian Noise.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Perturbação estocástica gaussiana 2D via transformação Box-Muller sem alocações no heap.
pub fn box_muller_noise(seed: u64, index: u64) -> (f64, f64) {
    let mut h1 = seed.wrapping_add(index.wrapping_mul(2).wrapping_add(1)).wrapping_mul(0x517c_c1b7_2722_0a95);
    h1 ^= h1 >> 33;
    let u1 = ((h1 & 0x001F_FFFF_FFFF_FFFF) as f64 + 1.0) / (0x0020_0000_0000_0000u64 as f64);

    let mut h2 = seed.wrapping_add(index.wrapping_mul(2).wrapping_add(2)).wrapping_mul(0x517c_c1b7_2722_0a95);
    h2 ^= h2 >> 33;
    let u2 = ((h2 & 0x001F_FFFF_FFFF_FFFF) as f64 + 1.0) / (0x0020_0000_0000_0000u64 as f64);

    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * PI * u2;
    (r * theta.cos(), r * theta.sin())
}

/// Aplica um ciclo de decaimento de Langevin na Bola de Poincaré para todos os nós 'EVOLVING' no SQLite.
///
/// x_{t+1} = proj_poincare( x_t - eta * Grad_V(x_t) + sqrt(2 * D * delta_t) * xi_t )
/// Com V(x) = 1/2 * ||x||^2 -> Grad_V(x) = x.
/// Nós com ||x|| >= 0.95 são marcados como 'SUPERSEDED'.
pub fn apply_langevin_decay(
    conn: &Connection,
    eta: f64,
    d_coeff: f64,
    delta_t: f64,
) -> Result<usize, String> {
    let mut stmt = conn
        .prepare("SELECT memory_id, poincare_x, poincare_y FROM souls_memory_nodes WHERE stability_status = 'EVOLVING'")
        .map_err(|e| format!("Erro ao preparar SELECT Langevin: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .map_err(|e| format!("Erro ao executar query Langevin: {}", e))?;

    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let noise_scale = (2.0 * d_coeff * delta_t).sqrt();
    let mut updated_count = 0;
    let mut updates = Vec::new();

    for (idx, res) in rows.enumerate() {
        let (mem_id, px, py) = match res {
            Ok(tuple) => tuple,
            Err(_) => continue,
        };

        // Grad_V(x) = (px, py)
        let grad_x = px;
        let grad_y = py;

        // Gerador de ruído gaussiano Box-Muller
        let (xi_x, xi_y) = box_muller_noise(now_epoch as u64, idx as u64);

        // x_{t+1} antes de projeção
        let next_x = px - eta * grad_x + noise_scale * xi_x;
        let next_y = py - eta * grad_y + noise_scale * xi_y;

        let (final_x, final_y) = proj_poincare((next_x, next_y));
        let norm = (final_x * final_x + final_y * final_y).sqrt();

        let new_status = if norm >= 0.95 {
            "SUPERSEDED"
        } else {
            "EVOLVING"
        };

        updates.push((mem_id, final_x, final_y, new_status));
    }

    drop(stmt);

    for (mem_id, fx, fy, status) in updates {
        let sql = "UPDATE souls_memory_nodes SET poincare_x = ?1, poincare_y = ?2, stability_status = ?3, updated_at = ?4 WHERE memory_id = ?5";
        if let Err(e) = conn.execute(sql, params![fx, fy, status, now_epoch, mem_id]) {
            eprintln!("[langevin_decay] ALERTA: Falha ao atualizar nó {}: {}", mem_id, e);
        } else {
            updated_count += 1;
        }
    }

    Ok(updated_count)
}
