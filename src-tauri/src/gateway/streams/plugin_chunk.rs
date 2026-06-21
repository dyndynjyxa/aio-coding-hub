//! Usage: Apply gateway response chunk plugin hooks before stream accounting.

use crate::gateway::plugins::context::{GatewayPluginHookName, GatewayStreamHookInput};
use crate::gateway::plugins::pipeline::GatewayPluginPipeline;
use axum::body::Bytes;
use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

type PluginChunkFuture =
    Pin<Box<dyn Future<Output = Result<Option<Bytes>, reqwest::Error>> + Send>>;

pub(super) const PLUGIN_STREAM_ERROR_MARKER: &str = ": aio-plugin-error\n";

pub(in crate::gateway) struct PluginChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    upstream: S,
    pipeline: Arc<GatewayPluginPipeline>,
    db: crate::db::Db,
    trace_id: String,
    sequence: u64,
    pending: Option<PluginChunkFuture>,
}

impl<S> PluginChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        pipeline: Arc<GatewayPluginPipeline>,
        db: crate::db::Db,
        trace_id: String,
    ) -> Self {
        Self {
            upstream,
            pipeline,
            db,
            trace_id,
            sequence: 0,
            pending: None,
        }
    }
}

pub(in crate::gateway) enum MaybePluginChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    Direct(S),
    WithPlugins(PluginChunkStream<S>),
}

impl<S> MaybePluginChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    pub(in crate::gateway) fn new(
        upstream: S,
        pipeline: Arc<GatewayPluginPipeline>,
        db: crate::db::Db,
        trace_id: String,
    ) -> Self {
        if pipeline.has_plugins_for_hook(GatewayPluginHookName::ResponseChunk) {
            Self::WithPlugins(PluginChunkStream::new(upstream, pipeline, db, trace_id))
        } else {
            Self::Direct(upstream)
        }
    }
}

impl<S> Stream for MaybePluginChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.as_mut().get_mut() {
            Self::Direct(upstream) => Pin::new(upstream).poll_next(cx),
            Self::WithPlugins(stream) => Pin::new(stream).poll_next(cx),
        }
    }
}

impl<S> Stream for PluginChunkStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin + Send + 'static,
{
    type Item = Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();

        loop {
            if let Some(mut pending) = this.pending.take() {
                match pending.as_mut().poll(cx) {
                    Poll::Ready(Ok(Some(chunk))) => return Poll::Ready(Some(Ok(chunk))),
                    Poll::Ready(Ok(None)) => return Poll::Ready(None),
                    Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                    Poll::Pending => {
                        this.pending = Some(pending);
                        return Poll::Pending;
                    }
                }
            }

            let chunk = match Pin::new(&mut this.upstream).poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => chunk,
                Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            };

            this.sequence = this.sequence.saturating_add(1);
            let pipeline = Arc::clone(&this.pipeline);
            let db = this.db.clone();
            let trace_id = this.trace_id.clone();
            let sequence = this.sequence;
            this.pending = Some(Box::pin(async move {
                let input = GatewayStreamHookInput {
                    trace_id: trace_id.clone(),
                    chunk,
                    sequence,
                };
                match pipeline.run_stream_hook(input).await {
                    Ok(output) => {
                        crate::gateway::plugins::audit::persist_gateway_plugin_audit_events(
                            &db,
                            &trace_id,
                            output.audit_events.clone(),
                        );
                        if let Some(blocked) = output.blocked {
                            tracing::warn!(
                                trace_id = %trace_id,
                                status = blocked.status,
                                reason = %blocked.reason,
                                "plugin blocked gateway stream chunk"
                            );
                            return Ok(Some(Bytes::from(format!(
                                "{PLUGIN_STREAM_ERROR_MARKER}event: error\ndata: {{\"error\":\"plugin_blocked\",\"reason\":{}}}\n\n",
                                serde_json::to_string(&blocked.reason)
                                    .unwrap_or_else(|_| "\"Plugin blocked gateway stream\"".to_string())
                            ))));
                        }
                        Ok(Some(output.chunk))
                    }
                    Err(mut err) => {
                        crate::gateway::plugins::audit::persist_gateway_plugin_error_audit_events(
                            &db, &trace_id, &mut err,
                        );
                        tracing::warn!(
                            trace_id = %trace_id,
                            error = %err,
                            "plugin stream hook failed"
                        );
                        Ok(Some(Bytes::from(format!(
                            "{PLUGIN_STREAM_ERROR_MARKER}event: error\ndata: {{\"error\":\"plugin_failed\",\"reason\":{}}}\n\n",
                            serde_json::to_string(&err.to_string())
                                .unwrap_or_else(|_| "\"Plugin stream hook failed\"".to_string())
                        ))))
                    }
                }
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plugins::{
        PluginDetail, PluginHook, PluginHostCompatibility, PluginInstallSource, PluginManifest,
        PluginPermissionRisk, PluginRuntime, PluginStatus, PluginSummary,
    };
    use crate::gateway::plugins::context::GatewayPluginHookName;
    use crate::gateway::plugins::pipeline::{
        GatewayPluginPipeline, GatewayPluginPipelineConfig, InMemoryGatewayPluginExecutor,
    };
    use std::sync::Arc;

    struct EmptyStream;

    impl Stream for EmptyStream {
        type Item = Result<Bytes, reqwest::Error>;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(None)
        }
    }

    fn plugin_for_hook(plugin_id: &str, hook_name: GatewayPluginHookName) -> PluginDetail {
        PluginDetail {
            summary: PluginSummary {
                id: 1,
                plugin_id: plugin_id.to_string(),
                name: plugin_id.to_string(),
                current_version: Some("1.0.0".to_string()),
                status: PluginStatus::Enabled,
                runtime: "declarativeRules".to_string(),
                permission_risk: PluginPermissionRisk::High,
                update_available: false,
                last_error: None,
                created_at: 1,
                updated_at: 1,
            },
            manifest: PluginManifest {
                id: plugin_id.to_string(),
                name: plugin_id.to_string(),
                version: "1.0.0".to_string(),
                api_version: "1.0.0".to_string(),
                runtime: PluginRuntime::DeclarativeRules {
                    rules: vec!["rules/main.json".to_string()],
                },
                hooks: vec![PluginHook {
                    name: hook_name.as_str().to_string(),
                    priority: 0,
                    failure_policy: Some("fail-open".to_string()),
                }],
                permissions: vec![],
                host_compatibility: PluginHostCompatibility {
                    app: ">=0.56.0 <1.0.0".to_string(),
                    plugin_api: "^1.0.0".to_string(),
                    platforms: vec![],
                },
                entry: None,
                config_schema: None,
                config_version: None,
                description: None,
                author: None,
                homepage: None,
                repository: None,
                license: None,
                checksum: None,
                signature: None,
                category: None,
            },
            install_source: PluginInstallSource::Local,
            installed_dir: None,
            config: serde_json::json!({}),
            granted_permissions: vec![],
            pending_permissions: vec![],
            audit_logs: vec![],
            runtime_failures: vec![],
        }
    }

    fn db() -> crate::db::Db {
        let dir = tempfile::tempdir().expect("tempdir");
        crate::db::init_for_tests(&dir.path().join("plugin-chunk-fast-path.db")).expect("test db")
    }

    #[test]
    fn maybe_plugin_chunk_stream_uses_direct_fast_path_without_chunk_plugins() {
        let pipeline = Arc::new(GatewayPluginPipeline::for_tests(
            vec![plugin_for_hook(
                "plugin.request",
                GatewayPluginHookName::RequestAfterBodyRead,
            )],
            Arc::new(InMemoryGatewayPluginExecutor::new()),
            GatewayPluginPipelineConfig::default(),
        ));

        let stream = MaybePluginChunkStream::new(
            EmptyStream,
            pipeline,
            db(),
            "trace-no-chunk-plugin".to_string(),
        );

        assert!(matches!(stream, MaybePluginChunkStream::Direct(_)));
    }

    #[test]
    fn maybe_plugin_chunk_stream_wraps_when_chunk_plugins_are_active() {
        let pipeline = Arc::new(GatewayPluginPipeline::for_tests(
            vec![plugin_for_hook(
                "plugin.chunk",
                GatewayPluginHookName::ResponseChunk,
            )],
            Arc::new(InMemoryGatewayPluginExecutor::new()),
            GatewayPluginPipelineConfig::default(),
        ));

        let stream = MaybePluginChunkStream::new(
            EmptyStream,
            pipeline,
            db(),
            "trace-with-chunk-plugin".to_string(),
        );

        assert!(matches!(stream, MaybePluginChunkStream::WithPlugins(_)));
    }
}
