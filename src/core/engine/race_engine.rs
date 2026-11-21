use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use std::fs;
use tracing::{info, warn};
use crate::core::engine::ModeEngine;
use crate::core::types::{ConnectionRequest, ExecutionPlan, RouteAttempt};

#[derive(Debug, Deserialize)]
struct ConfigFile {
    groups: Vec<GroupConfig>,
    default_routes: Vec<RouteAttempt>,
}

#[derive(Debug, Deserialize)]
struct GroupConfig {
    name: String,
    domains: Vec<String>,
    routes: Vec<RouteAttempt>,
}

pub struct RaceEngine {
    config: ConfigFile,
}

impl Default for RaceEngine {
    fn default() -> Self {
        let config = match fs::read_to_string("config.json") {
            Ok(data) => serde_json::from_str(&data).unwrap_or_else(|e| {
                warn!("Failed to parse config.json: {}. Using empty config.", e);
                ConfigFile { groups: vec![], default_routes: vec![RouteAttempt::Direct] }
            }),
            Err(_) => {
                info!("config.json not found, using default Direct config.");
                ConfigFile { groups: vec![], default_routes: vec![RouteAttempt::Direct] }
            }
        };
        Self { config }
    }
}

#[async_trait]
impl ModeEngine for RaceEngine {
    fn name(&self) -> &'static str { "RaceEngine (Config Driven)" }

    async fn evaluate(&self, req: &ConnectionRequest) -> Result<ExecutionPlan> {
        let domain = req.domain.as_deref().unwrap_or("").to_lowercase();
        
        if domain.is_empty() {
            return Ok(ExecutionPlan {
                candidates: self.config.default_routes.clone(),
                fallback: true,
            });
        }

        for group in &self.config.groups {
            for rule_domain in &group.domains {
                if domain.ends_with(rule_domain) {
                    return Ok(ExecutionPlan {
                        candidates: group.routes.clone(),
                        fallback: true,
                    });
                }
            }
        }

        Ok(ExecutionPlan {
            candidates: self.config.default_routes.clone(),
            fallback: true,
        })
    }
}