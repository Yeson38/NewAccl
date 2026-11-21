use anyhow::Result;
use async_trait::async_trait;
use crate::core::engine::ModeEngine;
use crate::core::types::{ConnectionRequest, ExecutionPlan, RouteAttempt, EchSource};

#[derive(Default)]
pub struct RaceEngine;

#[async_trait]
impl ModeEngine for RaceEngine {
    fn name(&self) -> &'static str { "RaceEngine (hakimi + watt)" }

    async fn evaluate(&self, req: &ConnectionRequest) -> Result<ExecutionPlan> {
        let domain = req.domain.as_deref().unwrap_or("");
        
        if domain.contains("github.com") {
            return Ok(ExecutionPlan {
                candidates: vec![
                    RouteAttempt::IpProbe { ips: None },
                    RouteAttempt::Direct,
                    RouteAttempt::Ech { ech: EchSource::Own },
                    RouteAttempt::Sni { sni: vec!["www.bing.com".to_string()] },
                    RouteAttempt::Proxy { node_id: "local-socks5".to_string(), protocol: "socks5".to_string() },
                ],
                fallback: true,
            });
        }

        Ok(ExecutionPlan {
            candidates: vec![RouteAttempt::Direct],
            fallback: false,
        })
    }
}