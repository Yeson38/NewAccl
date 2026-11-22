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
        
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("new-accl.log") {
            let _ = writeln!(file, "{}", visitor.msg);
        }
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

// 核心新增：修改系统代理
fn set_system_proxy(enable: bool, event_tx: &Sender<Event>) {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
        if let Ok(key) = hkcu.open_subkey_with_flags(path, KEY_WRITE) {
            if enable {
                let _ = key.set_value("ProxyEnable", &1u32);
                let _ = key.set_value("ProxyServer", &"127.0.0.1:8443");
                let _ = event_tx.send(Event::Log("Windows 系统代理已自动设置为 127.0.0.1:8443".into()));
            } else {
                let _ = key.set_value("ProxyEnable", &0u32);
                let _ = event_tx.send(Event::Log("Windows 系统代理已关闭".into()));
            }
        } else {
            let _ = event_tx.send(Event::Log("修改注册表失败，请以管理员身份运行。".into()));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enable;
        let _ = event_tx.send(Event::Log("非 Windows 系统，请手动设置代理为 127.0.0.1:8443".into()));
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
                    let _ = event_tx.send(Event::Log("正在启动代理...".into()));
                    
                    // 启动成功后，修改系统代理
                    set_system_proxy(true, &event_tx);
                    
                    let dispatcher = dispatcher.clone();
                    let event_tx_clone = event_tx.clone();
                    let running_flag = is_running.clone();

                    tokio::spawn(async move {
                        match crate::core::proxy_server::start_proxy_server(dispatcher).await {
                            Ok(_) => {
                                let _ = event_tx_clone.send(Event::Log("代理已正常退出。".into()));
                            }
                            Err(e) => {
                                let _ = event_tx_clone.send(Event::Log(format!("启动失败: 端口被占用 ({})", e)));
                            }
                        }
                        running_flag.store(false, Ordering::SeqCst);
                        let _ = event_tx_clone.send(Event::Status(false));
                    });
                }
                Command::StopProxy => {
                    // 停止时，恢复系统代理
                    set_system_proxy(false, &event_tx);
                    
                    let _ = event_tx.send(Event::Log("尝试停止代理...".into()));
                    is_running.store(false, Ordering::SeqCst);
                    let _ = event_tx.send(Event::Status(false));
                }
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}