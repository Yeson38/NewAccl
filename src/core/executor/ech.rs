use std::pin::Pin;
use std::future::Future;
use anyhow::Result;
use crate::core::executor::RaceCandidate;
use crate::core::types::{BoxedStream, ConnectionRequest};

#[derive(Clone)]
pub struct EchCandidate {
    pub req: ConnectionRequest,
}

impl RaceCandidate for EchCandidate {
    fn label(&self) -> String { "ECH".to_string() }

    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        Box::pin(async move {
            let domain = self.req.domain.unwrap_or_default();
            let addr = format!("{}:{}", domain, self.req.port);
            
            // 【核心修复】：在这里我们只做 TCP 连接。
            // 真实的 ECH 握手，将交由浏览器（或者本地 TUN 虚拟网卡）去发起。
            // 只要 TCP 能连上，就证明这条线路可用。
            let tcp = tokio::net::TcpStream::connect(addr).await?;
            
            Ok(("ECH".to_string(), Box::new(tcp) as BoxedStream))
        })
    }
}