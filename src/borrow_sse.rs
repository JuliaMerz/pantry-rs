//Credits to https://medium.com/@martyn_63493/rust-server-side-event-client-with-hyper-0-4-e470ca3ef761
use bytes::Bytes;
use hyper::{body::HttpBody, Request, Uri};

use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct SseClient {
    pub join_handle: tokio::task::JoinHandle<Result<()>>,
}

#[derive(Debug)]
pub enum Event<T> {
    Failed,
    Data(T),
    Shutdown,
}

impl SseClient {
    pub async fn spawn(url: Uri, tx: Sender<Event<Bytes>>, timeout_ms: u64) -> Result<Self> {
        let host = url.host().expect("Uri has no host");
        let port = url.port_u16().unwrap_or(80);
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(addr).await?;

        let (mut sender, conn) = hyper::client::conn::handshake(stream).await?;

        // Spawn the TCP connection, this will stay alive for the life of the
        // connection
        let conn_tx = tx.clone();
        tokio::task::spawn(async move {
            if let Err(_) = conn.await {
                conn_tx.send(Event::Failed).await.ok();
            }
            conn_tx.send(Event::Shutdown).await.ok();
        });

        // Set the HOST header to match the request URL
        let authority = url.authority().unwrap().clone();
        let req = Request::builder()
            .uri(url)
            .header(hyper::header::HOST, authority.as_str())
            .body(hyper::Body::empty())?;

        // Generate a future so we can monitor for timeout on the initial connection
        let work = sender.send_request(req);

        // Do the timeout on getting the headers, just in case SSE source is not a HTTP server
        let mut res =
            match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), work).await {
                Ok(result) => result?,
                Err(_) => {
                    return Err(Box::new(tokio::io::Error::new(
                        tokio::io::ErrorKind::TimedOut,
                        "Timeout",
                    )))
                }
            };

        Ok(Self {
            join_handle: tokio::spawn(async move {
                // Stream the body to the producer channel
                while let Some(next) = res.data().await {
                    let chunk = next?;
                    tx.send(Event::Data(chunk)).await?;
                }

                Ok(())
            }),
        })
    }
}
