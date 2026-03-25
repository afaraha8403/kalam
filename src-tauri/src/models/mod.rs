//! Data models for entries (notes, tasks, history; reminder times live on notes/tasks) and app logs.

mod entry;
mod log;

pub use entry::{Entry, Subtask};
