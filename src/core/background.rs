use crossbeam_channel::{Receiver, Sender};
use crate::ui::{Command, Event};
use std::sync::Arc;
use crate::core::engine::{EngineDispatcher, ModeEngine};
use crate::core::engine::race_engine::RaceEngine;

// 简易的日志拦截器，把 tracing 日志推给 UI
struct UiLogger {
    tx: Sender<Event>,
}

impl tracing::Subscriber for UiLogger {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool { true }
    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id { tracing::span::Id::from_u64(1) }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut visitor = LogVisitor { msg: String::new() };
        event.record(&mut visitor);
        let _ = self.tx.send(Event::Log(visitor.msg));
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

struct LogVisitor { msg: String }
impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.msg = format!("{:?}", value);
        }
    }
}

pub async fn run_background(cmd_rx: Receiver<Command>, event_tx: Sender<Event>) {
    // 注册日志钩子
    tracing::subscriber::set_global_default(UiLogger { tx: event_tx.clone() }).unwrap();
    
    // 初始化你的 Engine
    let engine: Box<dyn ModeEngine> = Box::new(RaceEngine::default());
    let dispatcher = Arc::new(EngineDispatcher::new(engine));
    
    // 生成证书
    crate::core::mitm::init_ca().expect("Failed to init CA");

    loop {
        if let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                Command::StartProxy => {
                    let _ = event_tx.send(Event::Status(true));
                    let _ = event_tx.send(Event::Log("尝试启动代理...".into()));
                    
                    let dispatcher = dispatcher.clone();
                    tokio::spawn(async move {
                        crate::core::proxy_server::start_proxy_server(dispatcher).await.unwrap();
                    });
                }
                Command::StopProxy => {
                    let _ = event_tx.send(Event::Status(false));
                    let _ = event_tx.send(Event::Log("代理已停止 (需要重启软件应用更改)".into()));
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}