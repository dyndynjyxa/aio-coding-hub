//! Built-in search backends.
//!
//! Each backend implements the [`SearchBackend`] trait and is selected at
//! runtime by the interceptor based on the current `AppSettings`. To add a
//! new backend, implement the trait in a new submodule and wire it up in
//! [`super::factory::build_backend`].

pub mod brave;
pub mod llm_backed;
pub mod metaso;
pub mod tavily;
