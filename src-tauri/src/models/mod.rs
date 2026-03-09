//! Data models for entries (notes, tasks, reminders, history) and app logs.

mod entry;
mod log;

pub use entry::{Entry, Subtask};
