//! Gerador de UUIDv7 no metal (RFC 4122 / RFC 9562).
//!
//! Garantias SOULS:
//! - Unix Epoch ms nos primeiros 48 bits (ordenação cronológica e lexicográfica).
//! - 12 bits de entropia em `rand_a` com versão 7 (`0x7000`).
//! - 62 bits de entropia em `rand_b` com variante RFC 4122 (`0x8000...`).
//! - Formato 36 caracteres: `8-4-4-4-12`.
//! - Semente dinâmica baseada em nanossegundos + contador atômico para garantir unicidade em sub-ms.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use tinyrand::{Rand, Seeded, StdRand};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn generate_uuid_v7() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let now = duration.as_millis() as u64;
    let nanos = duration.as_nanos() as u64;

    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let seed = nanos.wrapping_add(count.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut rng = StdRand::seed(seed);

    let rand_a = (rng.next_u64() & 0x0FFF) as u16; // 12 bits de entropia
    let rand_b = rng.next_u64();                  // 64 bits de entropia

    // UUIDv7 Layout (48-bit timestamp + 4-bit ver + 12-bit rand_a + 2-bit var + 62-bit rand_b):
    // time_high: 32 bits (bits 47..16)
    // time_low:  16 bits (bits 15..0)
    let time_high = (now >> 16) as u32;
    let time_low = (now & 0xFFFF) as u16;

    // Forçar versão 7 no rand_a (4 bits superiores)
    let formatted_rand_a = 0x7000 | rand_a;

    // Forçar variante RFC 4122 (bits 10xx xxxx) no byte superior do rand_b
    let formatted_rand_b_high = 0x8000_0000_0000_0000 | (rand_b & 0x3FFF_FFFF_FFFF_FFFF);

    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        time_high,
        time_low,
        formatted_rand_a,
        (formatted_rand_b_high >> 48) as u16,
        formatted_rand_b_high & 0x0000_FFFF_FFFF_FFFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_v7_format_and_version() {
        let uuid = generate_uuid_v7();
        assert_eq!(uuid.len(), 36, "UUID deve ter exatamente 36 caracteres");
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5, "UUID deve ter 5 partes separadas por hífens");
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);

        // Versão deve ser '7' no primeiro char da 3ª parte
        assert!(parts[2].starts_with('7'), "3ª parte deve começar com '7' (versão 7)");

        // Variante deve ser '8', '9', 'a' ou 'b' no primeiro char da 4ª parte
        let var_char = parts[3].chars().next().unwrap();
        assert!(
            ['8', '9', 'a', 'b'].contains(&var_char),
            "4ª parte deve ter variante RFC 4122 (8, 9, a, b), recebeu '{var_char}'"
        );
    }

    #[test]
    fn test_uuid_v7_chronological_ordering() {
        let uuid1 = generate_uuid_v7();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let uuid2 = generate_uuid_v7();

        assert!(
            uuid1 < uuid2,
            "UUIDs gerados sequencialmente devem respeitar a ordem cronológica/lexicográfica: {uuid1} < {uuid2}"
        );
    }
}
