use anyhow::Result;
use async_trait::async_trait;
use crate::core::engine::ModeEngine;
use crate::core::types::{ConnectionRequest, ExecutionPlan, RouteAttempt};

pub struct RuleEngine;

#[async_trait]
impl ModeEngine for RuleEngine {
    fn name(&self) -> &'static str { "RuleEngine (Clash)" }

    async fn evaluate(&self, _req: &ConnectionRequest) -> Result<ExecutionPlan> {
        // 模拟 Clash 风格：自上而下，命中即止，只返回单条
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