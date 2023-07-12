use hyper::server::conn::Http;
use serde::Deserialize;
use serde_json::json;
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};

#[derive(Debug, Deserialize)]
pub struct MockRequest {
    pub response: String,
    pub secret: String,
}

pub async fn mock_turnstile() -> (
    mpsc::Receiver<(MockRequest, oneshot::Sender<bool>)>,
    String,
    tokio::task::JoinHandle<()>,
) {
    let (tx, rx) = mpsc::channel(1);

    // Bind to a random port
    let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();

    let addr = listener.local_addr().unwrap();
    let addr = format!("http://{}", addr);

    // Wait for http requests
    let handle = tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            Http::new()
                .serve_connection(
                    socket,
                    hyper::service::service_fn(|req| {
                        let tx = tx.clone();
                        async move {
                            let (_, body) = req.into_parts();
                            let body = hyper::body::to_bytes(body).await.unwrap();
                            let req = serde_json::from_slice(body.to_vec().as_slice()).unwrap();
                            let (otx, orx) = oneshot::channel::<bool>();
                            tx.send((req, otx)).await.unwrap();
                            let response = orx.await.unwrap();
                            Ok::<_, hyper::Error>(hyper::Response::new(hyper::Body::from(
                                json!({
                                    "success": response,
                                })
                                .to_string(),
                            )))
                        }
                    }),
                )
                .await
                .unwrap();
        }
    });

    (rx, addr, handle)
}
