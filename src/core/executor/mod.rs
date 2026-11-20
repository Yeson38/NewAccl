use std::time::{Duration, Instant};
use std::pin::Pin;
use std::future::Future;
use anyhow::{anyhow, Result};
use futures_util::stream::{FuturesUnordered, Stream, StreamExt};
use tokio::sync::watch;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{info, warn, error};

use crate::core::types::RouteAttempt;

pub const RACE_SIZE: usize = 8;
pub const RACE_STAGGER_MS: u64 = 150;

pub trait RaceCandidate: Send + Sync + 'static {
    type Stream: AsyncRead + AsyncWrite + Unpin + Send;
    fn label(&self) -> String;
    fn connect(self) -> Pin<Box<dyn Future<Output = Result<Self::Stream>> + Send>>;
}

pub enum CandidateError {
    TcpLevel(anyhow::Error),
    TlsLevel(anyhow::Error),
}

pub struct Racer;

impl Racer {
    pub async fn race<S, C>(
        mut stream: S,
    ) -> Result<(C, C::Stream)>
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
                                                return (label, Err(anyhow!("stagger cancelled by previous failure")));
                                            }
                                        }
                                    }
                                    let start = Instant::now();
                                    let res = candidate.connect().await;
                                    (label, res.map(|s| (s, start.elapsed())))
                                }));
                            }
                            None => exhausted = true,
                        }
                    }
                    Some(res) = pending.next(), if !pending.is_empty() => {
                        match res {
                            Ok((label, Ok((stream, latency)))) => {
                                info!("Race winner: {} (latency: {:?})", label, latency);
                                // 注意：这里需要根据实际情况获取赢家，这里为了骨架编译通过，暂时抛出错误
                                // 实际实现中，C 需要是 Clone 的，或者通过其他方式将赢家和流返回
                                todo!("Return the winning candidate and its stream. Need a way to preserve the candidate that won.")
                            }
                            Ok((label, Err(e))) => {
                                warn!("Candidate {} failed: {}", label, e);
                                if is_ip_level_failure(&e) {
                                    let _ = failure_tx.send(true);
                                    first_start = None;
                                }
                            }
                            Err(_) => {
                                error!("Task panicked");
                            }
                        }
                    }
                }
            } else if !pending.is_empty() {
                if let Some(Ok((label, Ok((stream, latency))))) = pending.next().await {
                    info!("Race winner: {} (latency: {:?})", label, latency);
                    todo!("Return winner");
                }
            } else {
                break;
            }
        }

        Err(anyhow!("All candidates exhausted and failed"))
    }
}

fn is_ip_level_failure(_e: &anyhow::Error) -> bool {
    // 这里根据实际错误类型判断是否属于 TCP 层失败
    // 例如：TcpTimeout | TcpUnreachable | TcpRefused
    true
}