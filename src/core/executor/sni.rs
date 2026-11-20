use std::pin::Pin;
use std::future::Future;
use anyhow::Result;
use crate::core::executor::RaceCandidate;
use crate::core::types::{BoxedStream, ConnectionRequest};

pub struct SniCandidate {
    pub req: ConnectionRequest,
    pub fake_sni: String,
}

impl RaceCandidate for SniCandidate {
    fn label(&self) -> String { format!("FakeSNI({})", self.fake_sni) }

    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        Box::pin(async move {
            // TODO: 集成 rustls 修改 SNI 的握手逻辑，这里使用 TCP 流占位
            let addr = format!("{}:{}", self.req.domain.unwrap_or_default(), self.req.port);
            let stream = tokio::net::TcpStream::connect(addr).await?;
            Ok((self.label(), Box::new(stream) as BoxedStream))
        })
    }
}