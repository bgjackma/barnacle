use std::{fs, future::Future, io::Result};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream},
};

// Listens for incoming connections and passes them off to a handler
struct Listener {
    tcp_listener: TcpListener,
}

// Handles an HTTP request asynchronously
struct Handler {
    stream: BufStream<TcpStream>,
}

pub async fn run(tcp_listener: TcpListener, _shutdown: impl Future) -> Result<()> {
    // TODO: respect shutdown
    let mut listener = Listener { tcp_listener };
    listener.start_listening().await
}

impl Listener {
    async fn start_listening(&mut self) -> Result<()> {
        println!("Now listening for connections!");
        loop {
            let (socket, _) = self.tcp_listener.accept().await?;

            let mut handler = Handler {
                stream: BufStream::new(socket),
            };
            tokio::spawn(async move { handler.handle_connection().await });
        }
    }
}

impl Handler {
    async fn handle_connection(&mut self) -> Result<()> {
        let mut request_line = String::new();
        self.stream.read_line(&mut request_line).await?;

        let (status_line, file) = match request_line.trim() {
            "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "html/hello.html"),
            _ => ("HTTP/1.1 404 NOT FOUND", "html/404.html"),
        };
        let contents = fs::read_to_string(file).unwrap();
        let length = contents.len();

        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

        self.stream.write_all(response.as_bytes()).await?;
        self.stream.flush().await?;
        Ok(())
    }
}
