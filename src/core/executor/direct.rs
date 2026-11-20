use std::pin::Pin;
use std::future::Future;
use anyhow::Result;
use crate::core::executor::RaceCandidate;
use crate::core::types::{BoxedStream, ConnectionRequest};

pub struct DirectCandidate {
    pub req: ConnectionRequest,
}

impl RaceCandidate for DirectCandidate {
    fn label(&self) -> String { "Direct".to_string() }

    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        Box::pin(async move {
            let addr = format!("{}:{}", self.req.domain.unwrap_or_default(), self.req.port);
            let stream = tokio::net::TcpStream::connect(addr).await?;
            Ok(("Direct".to_string(), Box::new(stream) as BoxedStream))
        })
    }
}