use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::core::types::{ConnectionRequest, ExecutionPlan};

pub mod race_engine;
pub mod rule_engine;

#[async_trait]
pub trait ModeEngine: Send + Sync {
    fn name(&self) -> &'static str;
    async fn evaluate(&self, req: &ConnectionRequest) -> Result<ExecutionPlan>;
}

pub struct EngineDispatcher {
    current_engine: RwLock<Box<dyn ModeEngine>>,
}

impl EngineDispatcher {
    pub fn new(initial_engine: Box<dyn ModeEngine>) -> Self {
        Self { current_engine: RwLock::new(initial_engine) }
    }

    pub async fn switch_mode(&self, new_engine: Box<dyn ModeEngine>) {
        let mut engine = self.current_engine.write().await;
        tracing::info!("Switching engine to {}", new_engine.name());
        *engine = new_engine;
    }

    pub async fn dispatch(&self, req: &ConnectionRequest) -> Result<ExecutionPlan> {
        let engine = self.current_engine.read().await;
        engine.evaluate(req).await
    }
}