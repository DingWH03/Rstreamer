use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use bytes::Bytes;
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tokio::sync::{broadcast, Mutex};
use async_stream::stream;
use rustc_hash::FxHashSet;
use axum::{
    body::StreamBody,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use tokio::task::JoinHandle;
use crate::error::RStreamerError;

pub struct StreamingServer {
    port: u16,
    receiver: Arc<Mutex<broadcast::Receiver<Vec<u8>>>>,
    server_handle: Option<JoinHandle<()>>,
}

impl StreamingServer {
    pub fn new(port: u16, receiver: broadcast::Receiver<Vec<u8>>) -> Result<Self, RStreamerError> {
        Ok(Self {
            port,
            receiver: Arc::new(Mutex::new(receiver)),
            server_handle: None,
        })
    }

    pub fn start(&mut self) -> Result<(), RStreamerError> {
        let port = self.port;
        let receiver = Arc::clone(&self.receiver);

        let handle = tokio::spawn(async move {
            let app = Router::new().route(
                "/",
                get({
                    let receiver = Arc::clone(&receiver);
                    move || async move {
                        let receiver = receiver.lock().await.resubscribe();
                        stream_handler(receiver).await
                    }
                }),
            );

            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            tracing::info!("Streaming server running on http://{}", addr);

            if let Err(e) = axum::Server::bind(&addr)
                .serve(app.into_make_service())
                .with_graceful_shutdown(shutdown_signal())
                .await
            {
                tracing::error!("Server error: {}", e);
            }
        });

        self.server_handle = Some(handle);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), RStreamerError> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            tracing::info!("Streaming server stopped.");
        }
        Ok(())
    }
}

// 处理流的函数，默认使用hash计算重复帧
async fn stream_handler(
    receiver: broadcast::Receiver<Vec<u8>>,
) -> impl IntoResponse {
    const BOUNDARY: &str = "rstreamer_boundary";

    let frame_stream = BroadcastStream::new(receiver)
        .filter_map(|res| async move { res.ok() });

    // 使用 FxHashSet 存储已发送的帧
    let mut sent_frames = FxHashSet::default();

    let byte_stream = stream! {
        for await frame in frame_stream {
            // 使用 FxHashSet 判断当前帧是否已经发送过
            if sent_frames.contains(&frame) {
                continue; // 如果已发送过，则跳过
            }

            // 将当前帧加入已发送帧集合
            sent_frames.insert(frame.clone());

            let header = format!(
                "--{}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                BOUNDARY,
                frame.len()
            );
            yield Ok::<Bytes, Infallible>(Bytes::from(header));
            yield Ok(Bytes::from(frame));
            yield Ok(Bytes::from("\r\n"));
        }

        let end = format!("--{}--\r\n", BOUNDARY);
        yield Ok(Bytes::from(end));
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, format!("multipart/x-mixed-replace; boundary={}", BOUNDARY))],
        StreamBody::new(byte_stream),
    )
}

// // 不使用hash计算重复帧
// async fn stream_handler_nohash(
//     receiver: broadcast::Receiver<Vec<u8>>,
// ) -> impl IntoResponse {
//     const BOUNDARY: &str = "rstreamer_boundary";

//     let frame_stream = BroadcastStream::new(receiver)
//         .filter_map(|res| async move { res.ok() });

//     let byte_stream = stream! {
//         for await frame in frame_stream {
//             let header = format!(
//                 "--{}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
//                 BOUNDARY,
//                 frame.len()
//             );
//             yield Ok::<Bytes, Infallible>(Bytes::from(header));
//             yield Ok(Bytes::from(frame));
//             yield Ok(Bytes::from("\r\n"));
//         }
//         let end = format!("--{}--\r\n", BOUNDARY);
//         yield Ok(Bytes::from(end));
//     };

//     (
//         StatusCode::OK,
//         [(header::CONTENT_TYPE, format!("multipart/x-mixed-replace; boundary={}", BOUNDARY))],
//         StreamBody::new(byte_stream),
//     )
// }

async fn shutdown_signal() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!("Failed to install CTRL+C handler: {}", e);
    }
}
