pub mod ast;
pub mod context;
pub mod memory;
pub mod state_thinking;
pub mod sys;

pub use ast::observability;
pub mod context_compression;
pub mod lean_vacuum; // Facade de compatibilidade (ADR-030)
pub use state_thinking::memory_graph;
pub use state_thinking::thinking;
