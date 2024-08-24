use std::net::SocketAddr;

use hyper::{server::conn::http1, service::Service};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpListener, sync::mpsc};
use tokio_util::sync::CancellationToken;

use crate::factory::ServiceFactory;

pub async fn listen_to<M, S>(
    addr: SocketAddr,
    make: M,
    shutdown: CancellationToken,
) -> crate::Result<()>
where
    M: ServiceFactory<(), Service = S>,
    S: Service<crate::Request, Response = crate::Response> + Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: Send,
{
    let listener = TcpListener::bind(addr).await?;
    let (shutdown_tasks_tx, mut shutdown_tasks_rx) = mpsc::channel::<()>(1);
    while !shutdown.is_cancelled() {
        // Listen for new connections
        let conn = tokio::select! {
             res = listener.accept() => {res}
             _ = shutdown.cancelled() => { break; }
        };

        let (stream, _) = conn?;
        let io = TokioIo::new(stream);

        let service = make.get_service(());
        let task_shutdown_tx = shutdown_tasks_tx.clone();
        tokio::task::spawn(async move {
            // never called but dropped when it goes out of scope
            let _ = task_shutdown_tx;
            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
    // shutdown has beem signaled, drop and wait for all tasks to drop
    drop(shutdown_tasks_tx);
    shutdown_tasks_rx.recv().await;
    println!("All listener tasks completed.");
    Ok(())
}
