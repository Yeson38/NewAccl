use std::time::{Duration, Instant};
use std::pin::Pin;
use std::future::Future;
use anyhow::{anyhow, Result};
use futures_util::stream::{FuturesUnordered, Stream, StreamExt};
use tokio::sync::watch;
use tracing::{info, warn};
use crate::core::types::{BoxedStream, RouteAttempt, ConnectionRequest};

pub mod direct;
pub mod ech;
pub mod sni;
pub mod proxy;

pub const RACE_SIZE: usize = 8;
pub const RACE_STAGGER_MS: u64 = 150;

pub trait RaceCandidate: Send + Sync + 'static {
    fn label(&self) -> String;
    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>>;
}

pub enum Candidate {
    Direct(direct::DirectCandidate),
    Ech(ech::EchCandidate),
    Sni(sni::SniCandidate),
    Proxy(proxy::ProxyCandidate),
}

impl RaceCandidate for Candidate {
    fn label(&self) -> String {
        match self {
            Candidate::Direct(c) => c.label(),
            Candidate::Ech(c) => c.label(),
            Candidate::Sni(c) => c.label(),
            Candidate::Proxy(c) => c.label(),
        }
    }
    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>> {
        match self {
            Candidate::Direct(c) => c.connect(),
            Candidate::Ech(c) => c.connect(),
            Candidate::Sni(c) => c.connect(),
            Candidate::Proxy(c) => c.connect(),
        }
    }
}

pub struct Racer;

impl Racer {
    pub async fn race<S, C>(mut stream: S) -> Result<BoxedStream>
    where
        C: RaceCandidate,
        S: Stream<Item = C> + Unpin,
    {
        let mut pending = FuturesUnordered::new();
        let (failure_tx, failure_rx) = watch::channel(false);
        let mut first_start: Option<Instant> = None;
        let mut exhausted = false;

        loop {
            if !exhausted && pending.len() < RACE_SIZE {
                tokio::select! {
                    maybe_candidate = stream.next() => {
                        match maybe_candidate {
                            Some(candidate) => {
                                let now = Instant::now();
                                let start_at = match first_start {
                                    Some(start) => start + Duration::from_millis(RACE_STAGGER_MS),
                                    None => now,
                                };
                                first_start = Some(start_at);

                                let mut rx = failure_rx.clone();
                                let label = candidate.label();

                                pending.push(tokio::spawn(async move {
                                    let delay = start_at.saturating_duration_since(Instant::now());
                                    if !delay.is_zero() {
                                        tokio::select! {
                                            _ = tokio::time::sleep(delay) => {},
                                            _ = rx.wait_for(|v| *v) => {
                                                return Err(anyhow!("cancelled"));
                                            }
                                        }
                                    }
                                    let start = Instant::now();
                                    match candidate.connect().await {
                                        Ok((lbl, s)) => Ok((lbl, s, start.elapsed())),
                                        Err(e) => {
                                            let _ = label;
                                            Err(e)
                                        },
                                    }
                                }));
                            }
                            None => exhausted = true,
                        }
                    }
                    Some(res) = pending.next(), if !pending.is_empty() => {
                        if let Ok(Ok((label, stream, latency))) = res {
                            info!("Race winner: {} (latency: {:?})", label, latency);
                            return Ok(stream);
                        } else if let Ok(Err(e)) = res {
                            warn!("Candidate failed: {}", e);
                            let _ = failure_tx.send(true);
                            first_start = None;
                        }
                    }
                }
            } else if !pending.is_empty() {
                if let Some(Ok(Ok((label, stream, latency)))) = pending.next().await {
                    info!("Race winner (final): {} (latency: {:?})", label, latency);
                    return Ok(stream);
                }
            } else {
                break;
            }
        }
        Err(anyhow!("All candidates failed"))
    }
}

pub fn build_candidates(plan: crate::core::types::ExecutionPlan, req: ConnectionRequest) -> Vec<Candidate> {
    let mut cands: Vec<Candidate> = Vec::new();
    for attempt in plan.candidates {
        match attempt {
            RouteAttempt::Direct => {
                cands.push(Candidate::Direct(direct::DirectCandidate { req: req.clone() }));
            }
            RouteAttempt::Ech { .. } => {
                cands.push(Candidate::Ech(ech::EchCandidate { req: req.clone() }));
            }
            RouteAttempt::Sni { sni } => {
                for fake in sni {
                    cands.push(Candidate::Sni(sni::SniCandidate { req: req.clone(), fake_sni: fake }));
                }
            }
            RouteAttempt::Proxy { node_id, .. } => {
                cands.push(Candidate::Proxy(proxy::ProxyCandidate { req: req.clone(), node_id }));
            }
        }
    }
    cands
}