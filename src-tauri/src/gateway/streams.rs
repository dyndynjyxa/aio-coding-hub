//! Usage: Gateway stream adapters (gunzip, relays, usage/timing tees).

pub(crate) mod response;
pub(crate) use response::{UpstreamByteStream, UpstreamResponse, UpstreamStreamError};

mod types;
pub(super) use types::{StreamActivityTracker, StreamFinalizeCtx};

mod finalize;
mod request_end;

mod relay;
pub(super) use relay::{FirstChunkStream, RelayBodyStream};

mod gunzip;
pub(super) use gunzip::GunzipStream;

mod plugin_chunk;
pub(super) use plugin_chunk::{is_plugin_stream_error_chunk, MaybePluginChunkStream};

mod usage_tee;
pub(super) use usage_tee::{
    spawn_usage_sse_relay_body, UsageBodyBufferTeeStream, UsageSseTeeStream,
};

mod timing;
pub(super) use timing::TimingOnlyTeeStream;
