//! Usage: Gateway listen-address helpers kept as a compatibility facade.

pub(crate) use crate::shared::listen_address::{
    format_host_port, is_wildcard_host, parse_custom_listen_address,
};
