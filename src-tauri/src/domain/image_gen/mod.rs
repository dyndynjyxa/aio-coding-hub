//! Usage: Image generation adapter config persistence and pure HTTP transport helpers.
//!
//! The API key is read from SQLite and injected into outbound requests here; it
//! never crosses the IPC boundary in either direction.

mod config;
mod transport;

pub(crate) use config::{config_connection, config_get, config_set, ImageGenConfigView};
pub(crate) use transport::{
    fetch_image, post_json, post_multipart, ImageGenFetchedImage, ImageGenHttpResponse,
    ImageGenMultipartFile,
};

#[cfg(test)]
mod tests;
