//! Usage: WebDAV client for remote config sync (upload/download/test).

mod client;

#[cfg(test)]
mod tests;

pub use client::{
    webdav_download, webdav_test_connection, webdav_upload, WebDavConfig, WebDavTestResult,
    WebDavUploadResult,
};
