use bytes::Bytes;
use http::Method;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::Incoming;
use hyper::client::conn::http1::Builder as Http1Client;
use hyper::service::Service;
use hyper::upgrade::Upgraded;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::net::{lookup_host, TcpStream};

use crate::Error::RouteError;
use crate::Result;

pub struct Forward {
    target: SocketAddr,
}

impl Forward {
    // attempt to resolve target by request IP or simple DNS
    pub async fn from_req(request: &Request<Incoming>) -> Result<Forward> {
        let Some(authority) = request.uri().authority() else {
            return Err(RouteError(request.uri().to_string()));
        };
        let addrs: Vec<SocketAddr> = lookup_host(authority.to_string()).await?.collect();

        println!("Found addrs: {:?}", addrs);
        // just take the first one
        let Some(addr) = addrs.into_iter().next() else {
            return Err(RouteError(request.uri().to_string()));
        };
        Ok(Forward { target: addr })
    }
}

impl hyper::service::Service<Request<Incoming>> for Forward {
    type Response = crate::Response;
    type Error = crate::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>> + Send + 'static>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let target = self.target.clone();
        Box::pin(async move {
            println!("{:?}", req);
            if req.method() == Method::CONNECT {
                // Establish a CONNECT tunnel that can be upgraded to HTTP2 and/or TLS
                tokio::task::spawn(async move {
                    // Upgrade can only happen after we return an empty body below
                    match hyper::upgrade::on(req).await {
                        Ok(upgraded) => {
                            if let Err(e) = tunnel(upgraded, target).await {
                                eprintln!("server io error: {}", e);
                            };
                        }
                        Err(e) => {
                            eprintln!("upgrade error: {}", e)
                        }
                    }
                });
                // Return an empty body immediately
                Ok(Response::new(empty()))
            } else {
                // Forward all other HTTP requests
                let stream = TcpStream::connect(target).await?;
                let io = TokioIo::new(stream);
                let (mut sender, conn) = Http1Client::new()
                    .preserve_header_case(true)
                    .handshake(io)
                    .await?;

                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        println!("Connection failed: {:?}", err);
                    }
                });

                let resp = sender.send_request(req).await?;
                Ok(resp.map(|body| body.boxed()))
            }
        })
    }
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new().map_err(|x| match x {}).boxed()
}
// Open a tunnel over an upgrade-ready connection
async fn tunnel(upgraded: Upgraded, addr: SocketAddr) -> crate::Result<()> {
    // Connect to remote server
    println!("Local cx upgraded, attempting to connect to {:?}", addr);
    let mut server = TcpStream::connect(addr).await?;
    println!("connected");
    let mut upgraded = TokioIo::new(upgraded);

    let (from_client, from_server) =
        tokio::io::copy_bidirectional(&mut upgraded, &mut server).await?;

    println!(
        "client wrote {} bytes and received {} bytes",
        from_client, from_server
    );
    Ok(())
}

pub async fn forward(request: Request<Incoming>) -> Result<crate::Response> {
    // Lookup target address and bind to fwd
    let fwd = Forward::from_req(&request).await?;
    fwd.call(request).await
}
