//! Usage: Cross-cutting utilities shared across domains (low-level helpers, pure logic).

pub(crate) mod blocking;
pub(crate) mod circuit_breaker;
pub(crate) mod cli_key;
pub(crate) mod error;
pub(crate) mod fs;
pub(crate) mod http_body;
pub(crate) mod ipc_confirm;
pub(crate) mod listen_address;
pub(crate) mod mutex_ext;
pub(crate) mod security;
pub(crate) mod sqlite;
pub(crate) mod text;
pub(crate) mod time;
pub(crate) mod user_home;
