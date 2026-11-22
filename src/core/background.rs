use crossbeam_channel::{Receiver, Sender};
use crate::ui::{Command, Event};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::fs::OpenOptions;
use std::io::Write;
use crate::core::engine::{EngineDispatcher, ModeEngine};
use crate::core::engine::race_engine::RaceEngine;

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
        
        // 1. 写入文件
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("new-accl.log") {
            let _ = writeln!(file, "{}", visitor.msg);
        }
        
        // 2. 发送给 UI
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
    let _ = tracing::subscriber::set_global_default(UiLogger { tx: event_tx.clone() });
    
    let _ = event_tx.send(Event::Log("后台服务初始化...".into()));

    let engine: Box<dyn ModeEngine> = Box::new(RaceEngine::default());
    let dispatcher = Arc::new(EngineDispatcher::new(engine));
    
    if let Err(e) = crate::core::mitm::init_ca() {
        let _ = event_tx.send(Event::Log(format!("证书初始化失败: {}", e)));
    }

    let is_running = Arc::new(AtomicBool::new(false));

    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                Command::StartProxy => {
                    if is_running.load(Ordering::SeqCst) {
                        let _ = event_tx.send(Event::Log("代理已经在运行中。".into()));
                        continue;
                    }
                    
                    is_running.store(true, Ordering::SeqCst);
                    let _ = event_tx.send(Event::Status(true));
                    let _ = event_tx.send(Event::Log("正在尝试绑定端口 8443...".into()));
                    
                    let dispatcher = dispatcher.clone();
                    let event_tx_clone = event_tx.clone();
                    let running_flag = is_running.clone();

                    tokio::spawn(async move {
                        match crate::core::proxy_server::start_proxy_server(dispatcher).await {
                            Ok(_) => {
                                let _ = event_tx_clone.send(Event::Log("代理已正常退出。".into()));
                            }
                            Err(e) => {
                                let _ = event_tx_clone.send(Event::Log(format!("启动失败: 端口被占用或权限不足 ({})", e)));
                            }
                        }
                        running_flag.store(false, Ordering::SeqCst);
                        let _ = event_tx_clone.send(Event::Status(false));
                    });
                }
                Command::StopProxy => {
                    let _ = event_tx.send(Event::Log("尝试停止代理（注意：当前版本需要关闭软件来彻底释放端口）。".into()));
                    is_running.store(false, Ordering::SeqCst);
                    let _ = event_tx.send(Event::Status(false));
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}