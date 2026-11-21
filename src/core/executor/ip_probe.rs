use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;
use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use anyhow::{anyhow, Result};
use tokio::net::{TcpStream, lookup_host};
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use rustls::RootCertStore;
use webpki_roots::TLS_SERVER_ROOTS;
use tracing::debug;

use crate::core::executor::RaceCandidate;
use crate::core::types::{BoxedStream, ConnectionRequest};

#[derive(Clone)]
pub struct IpProbeCandidate {
    pub req: ConnectionRequest,
    pub ips: Option<Vec<IpAddr>>,
}

impl RaceCandidate for IpProbeCandidate {
    fn label(&self) -> String { "IpProbe".to_string() }

    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        Box::pin(async move {
            let domain = self.req.domain.clone().unwrap_or_default();
            if domain.is_empty() {
                return Err(anyhow!("IpProbe requires domain"));
            }

            let ips = match self.ips {
                Some(list) if !list.is_empty() => list,
                _ => resolve_domain(&domain).await?,
            };

            let mut probe_tasks = Vec::new();
            for ip in ips {
                let addr = SocketAddr::new(ip, self.req.port);
                probe_tasks.push(async move {
                    let start = std::time::Instant::now();
                    match timeout(Duration::from_secs(2), TcpStream::connect(addr)).await {
                        Ok(Ok(stream)) => Ok((addr, start.elapsed(), stream)),
                        Ok(Err(e)) => Err((addr, format!("TCP: {}", e))),
                        Err(_) => Err((addr, "timeout".to_string())),
                    }
                });
            }

            let results = futures_util::future::join_all(probe_tasks).await;
            let mut best_stream: Option<(SocketAddr, Duration, TcpStream)> = None;

            for res in results {
                match res {
                    Ok((addr, latency, stream)) => {
                        debug!("IpProbe {} latency {:?}", addr, latency);
                        if best_stream.is_none() || latency < best_stream.as_ref().unwrap().1 {
                            best_stream = Some((addr, latency, stream));
                        }
                    }
                    Err((addr, reason)) => {
                        debug!("IpProbe fail {}: {}", addr, reason);
                    }
                }
            }

            let (best_addr, best_latency, tcp) = best_stream.ok_or_else(|| anyhow!("All IP probes failed"))?;
            debug!("IpProbe winner {} latency {:?}", best_addr, best_latency);

            let mut roots = RootCertStore::empty();
            roots.extend(TLS_SERVER_ROOTS.iter().cloned());
            let config = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            let connector = TlsConnector::from(Arc::new(config));
            let server_name = rustls::pki_types::ServerName::try_from(domain)?;
            let tls_stream = connector.connect(server_name, tcp).await?;

            Ok((format!("IpProbe({})", best_addr.ip()), Box::new(tls_stream) as BoxedStream))
        })
    }
}

async fn resolve_domain(domain: &str) -> Result<Vec<IpAddr>> {
    let addrs: Vec<_> = lookup_host((domain, 443))
        .await?
        .map(|sa| sa.ip())
        .collect();
    if addrs.is_empty() {
        return Err(anyhow!("DNS returned no records for {}", domain));
    }
    Ok(addrs)
}