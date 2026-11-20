use anyhow::Result;
use async_trait::async_trait;
use crate::core::engine::ModeEngine;
use crate::core::types::{ConnectionRequest, ExecutionPlan, RouteAttempt};

pub struct RuleEngine;

#[async_trait]
impl ModeEngine for RuleEngine {
    fn name(&self) -> &'static str { "RuleEngine (Clash)" }

    async fn evaluate(&self, _req: &ConnectionRequest) -> Result<ExecutionPlan> {
        Ok(ExecutionPlan {
            candidates: vec![
                RouteAttempt::Proxy { 
                    node_id: "hk-01".to_string(), 
                    protocol: "trojan".to_string() 
                }
            ],
            fallback: false,
        })
    }
}