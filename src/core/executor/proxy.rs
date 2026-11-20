use std::pin::Pin;
use std::future::Future;
use anyhow::Result;
use crate::core::executor::RaceCandidate;
use crate::core::types::{BoxedStream, ConnectionRequest};

pub struct ProxyCandidate {
    pub req: ConnectionRequest,
    pub node_id: String,
}

impl RaceCandidate for ProxyCandidate {
    fn label(&self) -> String { format!("Proxy({})", self.node_id) }

    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        Box::pin(async move {
            // TODO: 集成 Watt Toolkit 节点池拨号逻辑
            // 实际应连接本地 Clash/Sing-box 的 socks5 端口
            let addr = format!("{}:{}", self.req.domain.unwrap_or_default(), self.req.port);
            let stream = tokio::net::TcpStream::connect(addr).await?;
            Ok((self.label(), Box::new(stream) as BoxedStream))
        })
    }
}