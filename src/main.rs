use std::{net::SocketAddr, str::FromStr};

use barnacle::{forward, listener::listen_to, Result};
use hyper::service::service_fn;
use tokio::signal;
use tokio_util::sync::CancellationToken;

#[tokio::main]
pub async fn main() -> Result<()> {
    let addr = SocketAddr::from_str("127.0.0.1:8181").expect("a valid IP");

    // Setup shutdown
    let shutdown = CancellationToken::new();
    let shutdown_on_signal = shutdown.clone();
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        println!("Received shutdown signal...");
        shutdown_on_signal.cancel();
    });

    // Run Server
    let service_fn = |()| service_fn(forward::forward);
    tokio::select! {
    res = listen_to(addr, service_fn, shutdown.clone()) => {
        if let Err(err) = res {
            println!("Error running server! {err}");
            println!("Shutting down...");
            shutdown.cancel();
        }
    }
        _ = shutdown.cancelled() => {  }
    }
    println!("Shutdown complete. Goodbye!");
    Ok(())
}
