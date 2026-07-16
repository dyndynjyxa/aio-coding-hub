//! Usage: Image generation adapter config persistence, pure HTTP transport
//! helpers, and task history persistence (files on disk + rows in SQLite).
//!
//! The API key is read from SQLite and injected into outbound requests here; it
//! never crosses the IPC boundary in either direction.

mod config;
mod history;
mod transport;

pub(crate) use config::{config_connection, config_get, config_set, ImageGenConfigView};
pub(crate) use history::{
    distinct_dirs, ensure_writable_dir, read_image, storage_cleanup, storage_dir_from_settings,
    storage_stats, task_delete, task_persist, tasks_clear, tasks_list, ImageGenStorageView,
    ImageGenTaskPersistPayload, ImageGenTaskRow,
};
pub(crate) use transport::{
    fetch_image, post_json, post_multipart, ImageGenFetchedImage, ImageGenHttpResponse,
    ImageGenMultipartFile,
};

#[cfg(test)]
mod tests;
