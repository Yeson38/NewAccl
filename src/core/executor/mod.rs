use std::time::{Duration, Instant};
use std::pin::Pin;
use std::future::Future;
use anyhow::{anyhow, Result};
use futures_util::stream::{FuturesUnordered, Stream, StreamExt};
use tokio::sync::watch;
use tracing::{info, warn};
use crate::core::types::BoxedStream;

pub mod direct;
pub mod ech;
pub mod sni;
pub mod proxy;

pub const RACE_SIZE: usize = 8;
pub const RACE_STAGGER_MS: u64 = 150;

pub trait RaceCandidate: Send + Sync + 'static {
    fn label(&self) -> String;
    // 返回 (赢家Label, 连接流)
    fn connect(self) -> Pin<Box<dyn Future<Output = Result<(String, BoxedStream)>> + Send>>;
}

pub struct Racer;

impl Racer {
    pub async fn race<S, C>(mut stream: S) -> Result<BoxedStream>
    where
        C: RaceCandidate,
        S: Stream<Item = C> + Unpin,
    {
        let mut pending = FuturesUnordered::new();
        let (failure_tx, mut failure_rx) = watch::channel(false);
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
                                        Err(e) => Err(e),
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
                            // 如果发生严重的 IP 级别错误，通知其余等待者放弃
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