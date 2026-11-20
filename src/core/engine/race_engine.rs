use anyhow::Result;
use async_trait::async_trait;
use crate::core::engine::ModeEngine;
use crate::core::types::{ConnectionRequest, ExecutionPlan, RouteAttempt, EchSource};

pub struct RaceEngine {
    // 实际项目中应从 JSON 读取配置
}

impl Default for RaceEngine {
    fn default() -> Self { Self {} }
}

#[async_trait]
impl ModeEngine for RaceEngine {
    fn name(&self) -> &'static str { "RaceEngine (hakimi)" }

    async fn evaluate(&self, req: &ConnectionRequest) -> Result<ExecutionPlan> {
        let domain = req.domain.as_deref().unwrap_or("");
        
        // 模拟 hakimi 规则匹配 (硬编码 github.com 示例)
        if domain.contains("github.com") {
            return Ok(ExecutionPlan {
                candidates: vec![
                    RouteAttempt::Direct,
                    RouteAttempt::Ech { ech: EchSource::Own },
                    RouteAttempt::Sni { sni: vec!["www.bing.com".to_string()] },
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