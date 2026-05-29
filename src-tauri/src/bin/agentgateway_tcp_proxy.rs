use std::io;
use std::net::SocketAddr;

use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};

fn parse_cli_args() -> (SocketAddr, SocketAddr) {
    let mut args = std::env::args();
    args.next();

    let mut listen = "127.0.0.1:3000".to_string();
    let mut upstream = "127.0.0.1:3001".to_string();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--listen" => {
                if let Some(v) = args.next() {
                    listen = v;
                }
            }
            "--upstream" => {
                if let Some(v) = args.next() {
                    upstream = v;
                }
            }
            _ => {}
        }
    }

    let listen = listen.parse().unwrap_or_else(|_| "127.0.0.1:3000".parse().unwrap());
    let upstream = upstream.parse().unwrap_or_else(|_| "127.0.0.1:3001".parse().unwrap());
    (listen, upstream)
}

fn set_nodelay(stream: &TcpStream) {
    let _ = stream.set_nodelay(true);
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let (listen, upstream) = parse_cli_args();
    let listener = TcpListener::bind(listen).await?;

    loop {
        let (mut downstream, _) = listener.accept().await?;
        tokio::spawn(async move {
            set_nodelay(&downstream);
            let Ok(mut up) = TcpStream::connect(upstream).await else {
                return;
            };
            set_nodelay(&up);
            let _ = copy_bidirectional(&mut downstream, &mut up).await;
        });
    }
}
