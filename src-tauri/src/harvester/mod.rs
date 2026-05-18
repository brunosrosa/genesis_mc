pub mod ramdisk;
pub mod git;
pub mod sandbox;
pub mod detect;
pub mod router;
pub mod sidecar;
pub mod extract;
pub mod community;
pub mod persist;
pub mod guard;
pub mod orchestrator;
pub mod canon;

pub(crate) const PHASE1_HEAVY_BLOB_MAX_CHARS: usize = 150_000;
