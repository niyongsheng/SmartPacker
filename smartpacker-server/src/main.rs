//! smartpacker-server 二进制入口:监听 `SMARTPACKER_ADDR`(默认 `0.0.0.0:5050`)。

use std::env;
use std::net::SocketAddr;

/// 启动 HTTP 服务。
#[tokio::main]
async fn main() {
    let addr: SocketAddr = env::var("SMARTPACKER_ADDR")
        .map(|s| s.parse().expect("SMARTPACKER_ADDR must be host:port"))
        .unwrap_or_else(|_| "0.0.0.0:5050".parse().expect("default addr"));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind listener");
    println!("smartpacker-server listening on http://{addr}");
    axum::serve(listener, smartpacker_server::app())
        .await
        .expect("serve");
}
