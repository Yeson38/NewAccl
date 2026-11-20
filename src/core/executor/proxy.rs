use std::pin::Pin;
use std::future::Future;
use anyhow::Result;
use crate::core::executor::RaceCandidate;
use crate::core::types::{BoxedStream, ConnectionRequest};

#[derive(Clone)]
pub struct ProxyCandidate {
    pub req: ConnectionRequest,
    pub node_id: String,
}

impl RaceCandidate for ProxyCandidate {
    fn label(&self) -> String { format!("Proxy({})", self.node_id) }

    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        Box::pin(async move {
            // 暂未接入真实节点池，先用 TCP 占位
            let addr = format!("{}:{}", self.req.domain.unwrap_or_default(), self.req.port);
            let stream = tokio::net::TcpStream::connect(addr).await?;
            Ok((self.label(), Box::new(stream) as BoxedStream))
        })
    }
}