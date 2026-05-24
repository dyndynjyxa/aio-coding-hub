//! Usage: Bounded HTTP response body readers for small JSON/text control-plane calls.

fn body_too_large_error(context: &str, limit: usize) -> String {
    format!("{context} body exceeds {limit} bytes")
}

pub(crate) async fn read_text_with_limit(
    mut response: reqwest::Response,
    limit: usize,
    context: &str,
) -> Result<String, String> {
    if response
        .content_length()
        .is_some_and(|len| len > limit as u64)
    {
        return Err(body_too_large_error(context, limit));
    }

    let capacity = response
        .content_length()
        .and_then(|len| usize::try_from(len).ok())
        .unwrap_or_default()
        .min(limit);
    let mut bytes = Vec::with_capacity(capacity);

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("{context} body read failed: {e}"))?
    {
        let remaining = limit.saturating_sub(bytes.len());
        if chunk.len() > remaining {
            return Err(body_too_large_error(context, limit));
        }
        bytes.extend_from_slice(&chunk);
    }

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn response_from_raw(raw_response: &'static [u8]) -> reqwest::Response {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("test server addr");
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf).await;
            stream
                .write_all(raw_response)
                .await
                .expect("write response");
        });

        let response = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("client")
            .get(format!("http://{addr}"))
            .send()
            .await
            .expect("send request");
        task.await.expect("server task");
        response
    }

    #[tokio::test]
    async fn read_text_with_limit_reads_body_under_limit() {
        let response = response_from_raw(
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
        )
        .await;

        let text = read_text_with_limit(response, 5, "test")
            .await
            .expect("read body");
        assert_eq!(text, "hello");
    }

    #[tokio::test]
    async fn read_text_with_limit_rejects_known_oversized_body() {
        let response = response_from_raw(
            b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabcdef",
        )
        .await;

        let err = read_text_with_limit(response, 5, "test")
            .await
            .expect_err("oversized body should fail");
        assert!(err.contains("test body exceeds 5 bytes"));
    }

    #[tokio::test]
    async fn read_text_with_limit_rejects_streamed_oversized_body() {
        let response =
            response_from_raw(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nabcdef").await;

        let err = read_text_with_limit(response, 5, "test")
            .await
            .expect_err("oversized streamed body should fail");
        assert!(err.contains("test body exceeds 5 bytes"));
    }
}
