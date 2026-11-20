use std::net::SocketAddr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ConnectionRequest {
    pub target_addr: SocketAddr,
    pub domain: Option<String>,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub candidates: Vec<RouteAttempt>,
    pub fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum RouteAttempt {
    Direct,
    Ech { ech: EchSource },
    Sni { sni: Vec<String> },
    Proxy { node_id: String, protocol: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EchSource {
    Own,
    Shared,
}

// 统一流定义，用于 Executor 层返回
pub trait AsyncStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> AsyncStream for T {}
pub type BoxedStream = Box<dyn AsyncStream>;