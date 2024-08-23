use crate::Error::RouteError;
use crate::Result;
use bytes::Bytes;
use http::Method;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::Incoming;
use hyper::client::conn::http1::Builder as Http1Client;
use hyper::upgrade::Upgraded;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::{lookup_host, TcpStream};

// Attempt to resolve target by request IP or simple DNS.
async fn get_target(request: &Request<Incoming>) -> Result<SocketAddr> {
    let Some(authority) = request.uri().authority() else {
        return Err(RouteError(request.uri().to_string()));
    };
    let addrs: Vec<SocketAddr> = lookup_host(authority.to_string()).await?.collect();

    println!("Found addrs: {:?}", addrs);
    // just take the first one
    let Some(addr) = addrs.into_iter().next() else {
        return Err(RouteError(request.uri().to_string()));
    };
    Ok(addr)
}

pub async fn forward(req: Request<Incoming>) -> Result<crate::Response> {
    println!("{:?}", req);
    let target = get_target(&req).await?;
    if req.method() == Method::CONNECT {
        // Establish a CONNECT tunnel that can be upgraded to HTTP2 and/or TLS
        tokio::task::spawn(async move {
            // Upgrade can only happen after we return an empty body below
            match hyper::upgrade::on(req).await {
                Ok(upgraded) => {
                    if let Err(e) = tunnel(upgraded, target).await {
                        eprintln!("server io error: {:?}", e);
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
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new().map_err(|x| match x {}).boxed()
}

async fn tunnel(upgraded: Upgraded, addr: SocketAddr) -> std::io::Result<()> {
    // Connect to remote server
    let server = TcpStream::connect(addr).await?;
    let upgraded = TokioIo::new(upgraded);
    let (mut server_rd, mut server_wr) = tokio::io::split(server);
    let (mut client_rd, mut client_wr) = tokio::io::split(upgraded);

    // asynchronous copies on this thread
    let up = tokio::io::copy(&mut client_rd, &mut server_wr);
    let down = tokio::io::copy(&mut server_rd, &mut client_wr);
    match tokio::try_join!(up, down) {
        Ok((up_bytes, down_bytes)) => {
            println!(
                "client wrote {} bytes and received {} bytes",
                up_bytes, down_bytes
            );
        }
        Err(e) => {
            println!("tunnel error: {}", e);
        }
    }
    Ok(())
}
