use barnacle::{forward, Result};
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpListener, signal};

#[tokio::main]
pub async fn main() -> Result<()> {
    let port = 8181;
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;

    // TODO: Cancellation
    //server::run(listener, signal::ctrl_c()).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .serve_connection(io, service_fn(forward::forward))
                .with_upgrades()
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
