//! Bridge MPSC do `souls_graph`.
//!
//! O canal `tokio::sync::mpsc` (buffer 100) é extendido a partir do padrão
//! `StateDbOp` existente em `souls_mcp_server.rs`. O worker consome operações
//! via `blocking_recv` em uma `std::thread` dedicada, mantendo `rusqlite`
//! síncrono e isolado do event loop do Tokio (sem `spawn_blocking`).
//!
//! Operações: CreateEntities, CreateRelations, AddObservations, Search,
//! OpenNodes, ReadGraph, DeleteEntities, DeleteObservations, DeleteRelations.

use crate::cognition::memory_graph::ops;
use crate::cognition::memory_graph::types::{Entity, ObservationInput, Relation};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot};

/// Erro interno do worker (não tipado pelo MCP; convertido no `handle_mcp`).
pub type WorkerResult = Result<Value, String>;

/// Envelope opaco das 9 operações canônicas.
pub enum MemGraphOp {
    CreateEntities {
        entities: Vec<Entity>,
        reply: oneshot::Sender<WorkerResult>,
    },
    CreateRelations {
        relations: Vec<Relation>,
        reply: oneshot::Sender<WorkerResult>,
    },
    AddObservations {
        observations: Vec<ObservationInput>,
        reply: oneshot::Sender<WorkerResult>,
    },
    Search {
        query: String,
        limit: usize,
        reply: oneshot::Sender<WorkerResult>,
    },
    OpenNodes {
        names: Vec<String>,
        reply: oneshot::Sender<WorkerResult>,
    },
    ReadGraph {
        limit: usize,
        reply: oneshot::Sender<WorkerResult>,
    },
    DeleteEntities {
        names: Vec<String>,
        reply: oneshot::Sender<WorkerResult>,
    },
    DeleteObservations {
        deletions: Vec<ObservationInput>,
        reply: oneshot::Sender<WorkerResult>,
    },
    DeleteRelations {
        relations: Vec<Relation>,
        reply: oneshot::Sender<WorkerResult>,
    },
}

/// Constrói o canal MPSC e devolve o `Sender` + dispara o worker dedicado.
pub fn spawn_memory_graph_worker(
    db_path: PathBuf,
) -> Result<mpsc::Sender<MemGraphOp>, Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel::<MemGraphOp>(100);

    std::thread::spawn(move || {
        let mut conn = match Connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[MemGraphWorker] ERRO ao abrir banco: {e}");
                return;
            }
        };
        // Tríade canônica SOULS: WAL, FK ON, busy_timeout 5s.
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.execute_batch("PRAGMA foreign_keys = ON;");
        let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));

        // Migração V1→V2 no boot do worker (idempotente, requer &mut).
        if let Err(e) = ops::migrate_v1_to_v2(&mut conn) {
            eprintln!("[MemGraphWorker] ERRO na migração V1→V2: {e}");
        }

        while let Some(op) = rx.blocking_recv() {
            match op {
                MemGraphOp::CreateEntities { entities, reply } => {
                    let r = ops::create_entities(&mut conn, &entities)
                        .map(|created| {
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("{} entidade(s) criada(s)/preservada(s).", created.len())
                                }]
                            })
                        })
                        .map_err(|e| e.to_string());
                    let _ = reply.send(r);
                }
                MemGraphOp::CreateRelations { relations, reply } => {
                    let r = ops::create_relations(&mut conn, &relations)
                        .map(|created| {
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("{} relação(ões) criada(s)/preservada(s).", created.len())
                                }]
                            })
                        })
                        .map_err(|e| e.to_string());
                    let _ = reply.send(r);
                }
                MemGraphOp::AddObservations { observations, reply } => {
                    let count: usize = observations.iter().map(|o| o.contents.len()).sum();
                    let r = ops::add_observations(&mut conn, &observations)
                        .map(|_| {
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("{count} observação(ões) adicionada(s).")
                                }]
                            })
                        })
                        .map_err(|e| e.to_string());
                    let _ = reply.send(r);
                }
                MemGraphOp::Search { query, limit, reply } => {
                    let r = ops::search_observations(&conn, &query, limit).map(|hits| {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&hits).unwrap_or_default()
                            }]
                        })
                    }).map_err(|e| e.to_string());
                    let _ = reply.send(r);
                }
                MemGraphOp::OpenNodes { names, reply } => {
                    let r = ops::open_nodes(&conn, &names).map(|nodes| {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&nodes).unwrap_or_default()
                            }]
                        })
                    }).map_err(|e| e.to_string());
                    let _ = reply.send(r);
                }
                MemGraphOp::ReadGraph { limit, reply } => {
                    let r = ops::read_graph(&conn, limit).map(|(entities, relations)| {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&json!({
                                    "entities": entities,
                                    "relations": relations
                                })).unwrap_or_default()
                            }]
                        })
                    }).map_err(|e| e.to_string());
                    let _ = reply.send(r);
                }
                MemGraphOp::DeleteEntities { names, reply } => {
                    let count = names.len();
                    let r = ops::delete_entities(&mut conn, &names).map(|_| {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("{count} entidade(s) removida(s) (cascade aplicado).")
                            }]
                        })
                    }).map_err(|e| e.to_string());
                    let _ = reply.send(r);
                }
                MemGraphOp::DeleteObservations { deletions, reply } => {
                    let count: usize = deletions.iter().map(|d| d.contents.len()).sum();
                    let r = ops::delete_observations(&mut conn, &deletions).map(|_| {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("{count} observação(ões) removida(s).")
                            }]
                        })
                    }).map_err(|e| e.to_string());
                    let _ = reply.send(r);
                }
                MemGraphOp::DeleteRelations { relations, reply } => {
                    let count = relations.len();
                    let r = ops::delete_relations(&mut conn, &relations).map(|_| {
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("{count} relação(ões) removida(s).")
                            }]
                        })
                    }).map_err(|e| e.to_string());
                    let _ = reply.send(r);
                }
            };
        }
    });

    Ok(tx)
}
