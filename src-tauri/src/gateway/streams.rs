//! Usage: Gateway stream adapters (gunzip, relays, usage/timing tees).

mod types;
pub(super) use types::StreamFinalizeCtx;

mod finalize;
mod request_end;

mod relay;
pub(super) use relay::{FirstChunkStream, RelayBodyStream};

mod gunzip;
pub(super) use gunzip::GunzipStream;

mod plugin_chunk;
pub(super) use plugin_chunk::MaybePluginChunkStream;

mod usage_tee;
pub(super) use usage_tee::{
    spawn_usage_sse_relay_body, UsageBodyBufferTeeStream, UsageSseTeeStream,
};

mod timing;
pub(super) use timing::TimingOnlyTeeStream;
