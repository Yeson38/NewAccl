mod core;
use core::engine::{EngineDispatcher, ModeEngine};
use core::engine::race_engine::RaceEngine;
use core::types::ConnectionRequest;
use tokio::net::TcpListener;
use tracing::{info, error};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    // 1. 初始化引擎 (这里默认使用 RaceEngine)
    let engine: Box<dyn ModeEngine> = Box::new(RaceEngine::default());
    let dispatcher = Arc::new(EngineDispatcher::new(engine));

    // 2. 启动混合端口入站 (Ingress)
    let listener = TcpListener::bind("127.0.0.1:8443").await?;
    info!("Fusion Accelerator listening on 127.0.0.1:8443");

    loop {
        let (mut inbound, _) = listener.accept().await?;
        let dispatcher = dispatcher.clone();

        tokio::spawn(async move {
            // 简易 HTTP CONNECT 解析 (用于测试)
            let mut buf = [0u8; 1024];
            if let Ok(n) = inbound.read(&mut buf).await {
                let req_str = String::from_utf8_lossy(&buf[..n]);
                if req_str.starts_with("CONNECT") {
                    let parts: Vec<&str> = req_str.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let host_port = parts[1];
                        let mut iter = host_port.split(':');
                        let domain = iter.next().unwrap_or("").to_string();
                        let port = iter.next().unwrap_or("443").parse::<u16>().unwrap_or(443);

                        let conn_req = ConnectionRequest {
                            target_addr: "0.0.0.0:0".parse().unwrap(), // 实际应解析 DNS
                            domain: Some(domain),
                            port,
                        };

                        // 核心：询问引擎
                        if let Ok(plan) = dispatcher.dispatch(&conn_req).await {
                            info!("Plan for {}: {:?}", conn_req.domain.unwrap(), plan);
                            // 这里应当调用 Executor 层去连接并转发
                            // 为了演示，直接返回 200 并关闭
                            let _ = inbound.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await;
                        }
                    }
                }
            }
        });
    }
}