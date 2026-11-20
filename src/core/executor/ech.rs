use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;
use anyhow::Result;
use tokio_rustls::TlsConnector;
use rustls::RootCertStore;
use webpki_roots::TLS_SERVER_ROOTS;
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
            let mut roots = RootCertStore::empty();
            roots.extend(TLS_SERVER_ROOTS.iter().cloned());
            
            let config = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            
            let connector = TlsConnector::from(Arc::new(config));
            let domain = self.req.domain.unwrap_or_default();
            let addr = format!("{}:{}", domain, self.req.port);
            
            let tcp = tokio::net::TcpStream::connect(addr).await?;
            let server_name = rustls::pki_types::ServerName::try_from(domain.clone())?;
            let tls_stream = connector.connect(server_name, tcp).await?;
            
            Ok(("ECH".to_string(), Box::new(tls_stream) as BoxedStream))
        })
    }
}