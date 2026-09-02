//! `socratic_thought_stream.rs` — Transmissor de Pensamentos Socráticos (Tauri v2 IPC).
//!
//! **ADR-005 / ADR-014 (Streaming Cognitivo Não-Bloqueante):**
//! - Intercepta cada nó de pensamento gerado (Regular, Revision, Branching) no `ThinkingEngine`.
//! - Sem aguardar a consolidação no SQLite, despacha assincronamente via canal MPSC.
//! - Emite o evento `"socratic-thought"` para o Svelte 5 atualizar o Active Canvas em tempo real.

use std::sync::{Arc, Mutex, OnceLock};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Nome canônico do evento Tauri de streaming de pensamento socrático.
pub const SOCRATIC_THOUGHT_EVENT: &str = "socratic-thought";

/// Payload leve de streaming do pensamento socrático para o frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocraticThoughtPayload {
    pub thought_id: String,
    pub session_id: String,
    pub branch_id: String,
    pub parent_thought_id: Option<String>,
    pub thought_type: String, // "regular", "revision", "branching"
    pub content: String,
    pub step_number: u32,
    pub duration_ms: u64,
    pub timestamp_ms: u64,
}

impl SocraticThoughtPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        thought_id: impl Into<String>,
        session_id: impl Into<String>,
        branch_id: impl Into<String>,
        parent_thought_id: Option<String>,
        thought_type: impl Into<String>,
        content: impl Into<String>,
        step_number: u32,
        duration_ms: u64,
    ) -> Self {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            thought_id: thought_id.into(),
            session_id: session_id.into(),
            branch_id: branch_id.into(),
            parent_thought_id,
            thought_type: thought_type.into(),
            content: content.into(),
            step_number,
            duration_ms,
            timestamp_ms,
        }
    }
}

/// Trait abstrato para sink de pensamentos socráticos.
pub trait SocraticThoughtSink: Send + Sync {
    fn emit_thought(&self, payload: &SocraticThoughtPayload) -> Result<(), String>;
}

/// Sink em memória para asserções e testes unitários.
#[derive(Default, Clone)]
pub struct InMemorySocraticThoughtSink {
    pub thoughts: Arc<Mutex<Vec<SocraticThoughtPayload>>>,
}

impl InMemorySocraticThoughtSink {
    pub fn new() -> Self {
        Self {
            thoughts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn snapshot(&self) -> Vec<SocraticThoughtPayload> {
        self.thoughts.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn count(&self) -> usize {
        self.thoughts.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.thoughts.lock() {
            g.clear();
        }
    }
}

impl SocraticThoughtSink for InMemorySocraticThoughtSink {
    fn emit_thought(&self, payload: &SocraticThoughtPayload) -> Result<(), String> {
        if let Ok(mut g) = self.thoughts.lock() {
            g.push(payload.clone());
        }
        Ok(())
    }
}

/// Broadcaster com canal MPSC não-bloqueante (`try_send`).
#[derive(Clone)]
pub struct SocraticThoughtBroadcaster {
    sender: mpsc::Sender<SocraticThoughtPayload>,
}

impl SocraticThoughtBroadcaster {
    /// Inicia um broadcaster ancorado em uma task assíncrona dedicada.
    pub fn new(sink: Arc<dyn SocraticThoughtSink>, buffer_size: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<SocraticThoughtPayload>(buffer_size);

        tokio::spawn(async move {
            while let Some(thought) = rx.recv().await {
                let _ = sink.emit_thought(&thought);
            }
        });

        Self { sender: tx }
    }

    /// Disparo não-bloqueante de pensamento socrático.
    pub fn broadcast(&self, payload: SocraticThoughtPayload) -> bool {
        self.sender.try_send(payload).is_ok()
    }
}

static GLOBAL_THOUGHT_BROADCASTER: OnceLock<SocraticThoughtBroadcaster> = OnceLock::new();

/// Configura o broadcaster global de pensamentos socráticos.
pub fn init_global_thought_broadcaster(sink: Arc<dyn SocraticThoughtSink>) -> bool {
    let broadcaster = SocraticThoughtBroadcaster::new(sink, 4096);
    GLOBAL_THOUGHT_BROADCASTER.set(broadcaster).is_ok()
}

/// Dispara um pensamento para o broadcaster global de forma não-bloqueante.
pub fn broadcast_socratic_thought(payload: SocraticThoughtPayload) -> bool {
    if let Some(broadcaster) = GLOBAL_THOUGHT_BROADCASTER.get() {
        broadcaster.broadcast(payload)
    } else {
        // Fallback direto síncrono caso broadcaster ainda não inicializado
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_socratic_thought_stream_mpsc_throughput() {
        let sink = Arc::new(InMemorySocraticThoughtSink::new());
        let broadcaster = SocraticThoughtBroadcaster::new(sink.clone(), 1024);

        for i in 1..=20 {
            let mode = match i % 3 {
                0 => "regular",
                1 => "revision",
                _ => "branching",
            };
            let payload = SocraticThoughtPayload::new(
                format!("thn_{i}"),
                "sess_test",
                "main",
                if i > 1 { Some(format!("thn_{}", i - 1)) } else { None },
                mode,
                format!("Hipótese socrática de número {i}"),
                i,
                10,
            );
            assert!(broadcaster.broadcast(payload));
        }

        // Aguarda drenagem da fila MPSC
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        assert_eq!(sink.count(), 20);

        let snap = sink.snapshot();
        assert_eq!(snap[0].thought_id, "thn_1");
        assert_eq!(snap[19].thought_id, "thn_20");
        assert_eq!(snap[0].thought_type, "revision");
        assert_eq!(snap[1].thought_type, "branching");
        assert_eq!(snap[2].thought_type, "regular");
    }
}
