use std::net::SocketAddr;

use hyper::{server::conn::http1, service::Service};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::factory::ServiceFactory;

pub async fn listen_to<M, S>(addr: SocketAddr, make: M) -> crate::Result<()>
where
    M: ServiceFactory<(), Service = S>,
    S: Service<crate::Request, Response = crate::Response> + Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
    S::Future: Send,
{
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        let service = make.get_service(());
        tokio::task::spawn(async move {
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
}

/*
pub async fn listen_to<M, S>(addr: SocketAddr, make: M) -> crate::Result<()>
where
    M: ServiceFactory<(), S>,
    // TYPE BOUNDS?!?
{
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let service = make.get_service(());
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

*/
