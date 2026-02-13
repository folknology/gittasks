//! Storage layer for task files

pub mod file_store;
pub mod id_generator;
pub mod location;
pub mod registry;

pub use file_store::{
    AggregatedTask, FileStore, FileStoreError, TaskFilter, TaskStats, list_aggregated,
    resolve_qualified_id,
};
pub use id_generator::IdGenerator;
pub use location::{TaskLocation, TaskLocationError};
pub use registry::{ProjectRegistry, ProjectStatus, RegistryError};
