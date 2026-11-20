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
pub struct SniCandidate {
    pub req: ConnectionRequest,
    pub fake_sni: String,
}

impl RaceCandidate for SniCandidate {
    fn label(&self) -> String { format!("FakeSNI({})", self.fake_sni) }

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
            
            let server_name = if self.fake_sni.is_empty() {
                rustls::pki_types::ServerName::IpAddress(std::net::Ipv4Addr::UNSPECIFIED.into())
            } else {
                rustls::pki_types::ServerName::try_from(self.fake_sni.clone())?
            };
            
            let tls_stream = connector.connect(server_name, tcp).await?;
            Ok((self.label(), Box::new(tls_stream) as BoxedStream))
        })
    }
}