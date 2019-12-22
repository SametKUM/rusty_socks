#![warn(rust_2018_idioms)]

mod connection;
mod handshake;
mod helpers;
mod request;
mod pump;
mod buffer_pool;

use tokio::net::TcpListener;
//use tokio::prelude::*;
use toml::from_str;
use serde::Deserialize;

use connection::Connection;
use helpers::Helpers;
use buffer_pool::BufferPool;

#[derive(Deserialize)]
struct Config {
    listen_interface: Option<String>,
    endpoint_interface: Option<String>,
    port: Option<u16>,
    buffer_size: Option<usize>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut config: Option<Config> = None;
    
    // Compute options.

    if args.len() == 2 {
        let config_file = args[1].to_owned();
        let config_file_data = tokio::fs::read(config_file).await?.to_owned();
        let config_text = std::str::from_utf8(&config_file_data)?;

        config = Some(from_str::<Config>(config_text)?);
    }

    let mut listen_interface: Option<String> = None;
    let mut endpoint_interface: Option<String> = None;
    let mut port = 1080u16;
    let mut buffer_size = 8192usize;
    
    if let Some(c) = config {
        listen_interface = c.listen_interface;
        endpoint_interface = c.endpoint_interface;
        port = c.port.unwrap_or(port);
        buffer_size = c.buffer_size.unwrap_or(buffer_size);
    }

    let listen_ip = match &listen_interface {
        Some(i) => Helpers::get_interface_ip(i)?.to_string(),
        None => "0.0.0.0".to_owned()
    };

    let endpoint_ip = match &endpoint_interface {
        Some(i) => Helpers::get_interface_ip(i)?.to_string(),
        None => "0.0.0.0".to_owned()
    };

    // Create a buffer pool (doubled so that each half of the connection achieves the desired size).
    let mut pool = BufferPool::new(2 * buffer_size);

    // Start the server.
    let mut listener = TcpListener::bind(format!("{}:{}", listen_ip, port)).await?;
    println!("Listening on tcp://{}:{} ... ", listen_ip, port);

    // Server loop.
    loop {
        println!("Buffer pool: {} leased / {} total.", pool.leased_count(), pool.total_count());
        println!("Awaiting new connection ...");

        let (stream, _) = listener.accept().await?;

        // TODO: Converting endpoint_interface to owned is a cop out.
        // Instead, we could compute the lifetimes correctly...
        Connection::from(stream, endpoint_ip.to_owned(), pool.lease()).handle();
    }
}