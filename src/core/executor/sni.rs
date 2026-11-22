use std::pin::Pin;
use std::future::Future;
use anyhow::Result;
use crate::core::executor::RaceCandidate;
use crate::core::types::{BoxedStream, ConnectionRequest};

#[derive(Clone)]
pub struct SniCandidate {
    pub req: ConnectionRequest,
    pub fake_sni: String,
}

impl RaceCandidate for SniCandidate {
    fn label(&self) -> String { format!("FakeSNI({})", self.fake_sni) }

    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        Box::pin(async move {
            let label = self.label();
            let domain = self.req.domain.unwrap_or_default();
            let addr = format!("{}:{}", domain, self.req.port);
            
            // 【核心修复】：只做 TCP 连接，将伪装 SNI 的重任留给真正负责加密的底层。
            let tcp = tokio::net::TcpStream::connect(addr).await?;
            
            Ok((label, Box::new(tcp) as BoxedStream))
        })
    }
}