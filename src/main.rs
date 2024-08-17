use barnacle::{server, Result};
use tokio::{net::TcpListener, signal};

#[tokio::main]
pub async fn main() -> Result<()> {
    let port = 8181;

    // Bind a TCP listener
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port)).await?;

    server::run(listener, signal::ctrl_c()).await?;
    Ok(())
}
