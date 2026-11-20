mod core;
use core::engine::{EngineDispatcher, ModeEngine};
use core::engine::race_engine::RaceEngine;
use core::proxy_server::start_proxy_server;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    // 1. 初始化引擎 (默认使用 RaceEngine)
    let engine: Box<dyn ModeEngine> = Box::new(RaceEngine::default());
    let dispatcher = Arc::new(EngineDispatcher::new(engine));

    // 2. 生成并加载本地 CA 证书
    core::mitm::init_ca().expect("Failed to init local CA");

    // 3. 启动 HTTP 代理入站
    info!("Starting NewAccl proxy on 127.0.0.1:8443...");
    start_proxy_server(dispatcher).await
}