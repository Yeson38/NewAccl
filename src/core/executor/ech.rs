use std::pin::Pin;
use std::future::Future;
use anyhow::Result;
use crate::core::executor::RaceCandidate;
use crate::core::types::{BoxedStream, ConnectionRequest};

pub struct EchCandidate {
    pub req: ConnectionRequest,
}

impl RaceCandidate for EchCandidate {
    fn label(&self) -> String { "ECH".to_string() }

    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        Box::pin(async move {
            // TODO: 集成 rustls ECH 握手逻辑，这里使用 TCP 流占位
            let addr = format!("{}:{}", self.req.domain.unwrap_or_default(), self.req.port);
            let stream = tokio::net::TcpStream::connect(addr).await?;
            Ok(("ECH".to_string(), Box::new(stream) as BoxedStream))
        })
    }
}