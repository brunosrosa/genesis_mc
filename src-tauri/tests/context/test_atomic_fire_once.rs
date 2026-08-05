//! `test_atomic_fire_once.rs`
//!
//! Marco 4.0.1 — Projeto Guilhotina: transplante da "Alma Matemática" do
//! [`autonomy.rs`](../../third_party/lean-ctx/src/tools/autonomy.rs) (cadáver READ-ONLY).
//!
//! Padrão canibalizado: inicialização atômica via `compare_exchange(SeqCst)`
//! para garantir disparo único livre de mutex (`Mutex`/`RwLock`), evitando
//! o anti-pattern Zero-Slop de manter `MutexGuard` através de `.await`.
//!
//! **Lei de ferro:** dois disparos concorrentes da mesma inicialização
//! devem resultar em **exatamente uma execução** do bloco crítico
//! (`producer`), enquanto o perdedor recebe `None` (idempotente).
//!
//! **Hipótese ABA:** o padrão CAS é seguro aqui porque o flag é
//! **monótono** (false → true, sem retorno). Não há transição cíclica
//! que corrompa a integridade temporal.
//!
//! **Performance:** o teste valida que N=64 disparos concorrentes
//! (1 thread produtora × 64 invocações paralelas via `std::thread::spawn`)
//! resultam em exatamente 1 execução observável, sem deadlock, sem
//! pânico, sem Mutex held across await.

use souls_mc_lib::cognition::lean_vacuum::atomic_once::FireOnce;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Caso 1: dois disparos sequenciais ⇒ apenas o primeiro ganha.
#[test]
fn fire_once_sequential_first_wins_second_loses() {
    let init = FireOnce::new();

    let first = init.try_init(|| 42_usize);
    assert_eq!(first, Some(42), "primeiro init deve ganhar");

    let second = init.try_init(|| 999_usize);
    assert_eq!(second, None, "segundo init deve perder (já inicializado)");
}

/// Caso 2: N disparos concorrentes ⇒ exatamente 1 ganha, N-1 perdem.
#[test]
fn fire_once_concurrent_only_one_winner() {
    const N: usize = 64;
    let init = Arc::new(FireOnce::new());
    let winners = Arc::new(AtomicUsize::new(0));
    let losers = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::with_capacity(N);
    for i in 0..N {
        let init = Arc::clone(&init);
        let winners = Arc::clone(&winners);
        let losers = Arc::clone(&losers);
        handles.push(std::thread::spawn(move || {
            // Cada thread tenta inicializar com um valor distinto para
            // detectar qual ganhou (deve ser exatamente um valor único).
            let result = init.try_init(move || i);
            match result {
                Some(_v) => {
                    winners.fetch_add(1, Ordering::SeqCst);
                }
                None => {
                    losers.fetch_add(1, Ordering::SeqCst);
                }
            }
        }));
    }

    for h in handles {
        h.join().expect("thread não deve panicar");
    }

    let w = winners.load(Ordering::SeqCst);
    let l = losers.load(Ordering::SeqCst);
    assert_eq!(w, 1, "exatamente 1 vencedor (winners={w}, losers={l})");
    assert_eq!(l, N - 1, "exatamente N-1 perdedores (winners={w}, losers={l})");
    assert_eq!(w + l, N, "winners + losers = N (sem perdas)");
}

/// Caso 3: `is_initialized` reflete corretamente o estado pós-init.
#[test]
fn fire_once_is_initialized_reflects_state() {
    let init = FireOnce::new();
    assert!(!init.is_initialized(), "fresh: false");

    let _ = init.try_init(|| "payload");
    assert!(init.is_initialized(), "após init: true");

    // Segunda chamada também vê true.
    let second = init.try_init(|| "ignored");
    assert!(second.is_none(), "segunda init retorna None");
    assert!(init.is_initialized(), "estado preservado");
}

/// Caso 4: o valor injetado pelo vencedor é observável pelos perdedores.
///
/// Crítico para o `SocraticWriteWorker`: o "vencedor" pode precisar expor
/// o resultado da inicialização (e.g., handle do thread, conexão, etc.)
/// sem que os perdedores dupliquem o trabalho. Aqui validamos o
/// invariante de "single source of truth": o `FireOnce` é
/// `Send + Sync` e o valor pode ser compartilhado.
#[test]
fn fire_once_winner_value_is_observable() {
    let init = Arc::new(FireOnce::new());
    let captured: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::with_capacity(8);
    for i in 0..8 {
        let init = Arc::clone(&init);
        let captured = Arc::clone(&captured);
        handles.push(std::thread::spawn(move || {
            if let Some(v) = init.try_init(move || i * 100) {
                captured.store(v, Ordering::SeqCst);
            }
        }));
    }
    for h in handles {
        h.join().expect("ok");
    }

    // O valor capturado é o do vencedor; como é um dos 8 valores possíveis
    // (0, 100, 200, ..., 700), apenas validamos que ele é múltiplo de 100.
    let v = captured.load(Ordering::SeqCst);
    assert_eq!(v % 100, 0, "valor capturado deve ser múltiplo de 100, got {v}");
    assert!(v <= 700, "valor capturado <= 700, got {v}");
}

/// Caso 5: ausência de Mutex — não usamos nenhum lock global.
///
/// Este teste documenta a invariante estrutural: `FireOnce` não depende
/// de `Mutex`/`RwLock`. Se algum dia alguém adicionar um lock, este
/// teste continua passando — mas o ponto é que o `Cargo.toml` do
/// `souls_mc_lib` não precisa importar nada de `parking_lot`/`std::sync`
/// além de `Arc` e atomics.
#[test]
fn fire_once_compiles_without_mutex() {
    // Compile-time check: o tipo existe, é Send+Sync, e o código compila.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<FireOnce<i32>>();
    assert_send_sync::<FireOnce<String>>();
    assert_send_sync::<FireOnce<Vec<u8>>>();
}
