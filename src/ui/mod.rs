use mitsuami::prelude::*;
use crossbeam_channel::unbounded;
use std::thread;
use std::sync::{Arc, Mutex};

pub enum Command {
    StartProxy,
    StopProxy,
}

pub enum Event {
    Log(String),
    Status(bool),
}

pub fn run_ui() {
    let (cmd_tx, cmd_rx) = unbounded::<Command>();
    let (event_tx, event_rx) = unbounded::<Event>();

    let logs_arc = Arc::new(Mutex::new(Vec::<String>::new()));
    let logs_clone = logs_arc.clone();

    // 后台运行逻辑
    let event_tx_clone = event_tx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(crate::core::background::run_background(cmd_rx, event_tx_clone));
    });

    // UI 主循环
    App::new()
        .window("NewAccl 加速器", Size::new(650.0, 500.0), move || {
            // 轮询处理通道事件
            while let Ok(event) = event_rx.try_recv() {
                match event {
                    Event::Log(msg) => {
                        let mut logs = logs_clone.lock().unwrap();
                        logs.push(format!("> {}", msg));
                        // 保持日志数量，防止内存爆炸
                        if logs.len() > 100 { logs.remove(0); }
                    }
                    Event::Status(_is_running) => {
                        // 状态控制，预留
                    }
                }
            }

            let logs_snapshot = logs_clone.lock().unwrap().join("\n");
            let cmd_tx_start = cmd_tx.clone();
            let cmd_tx_stop = cmd_tx.clone();
            
            view! {
                <Column gap=Spacing::Lg padding=Spacing::Xl>
                    // 顶部状态栏
                    <Row gap=Spacing::Md>
                        <Text text_style=TextStyle::Title>"NewAccl"</Text>
                        <Text text_style=TextStyle::Body>"● 状态正常"</Text>
                    </Row>

                    // 操作按钮区
                    <Row gap=Spacing::Md>
                        <Button @click=move || {
                            let _ = cmd_tx_start.send(Command::StartProxy);
                        }>"启动代理"</Button>
                        
                        <Button @click=move || {
                            let _ = cmd_tx_stop.send(Command::StopProxy);
                        }>"停止代理"</Button>
                    </Row>

                    // 提示区域
                    <Text text_style=TextStyle::Caption>
                        "提示：请在系统设置中将 HTTP 代理设置为 127.0.0.1:8443 以接管流量。"
                    </Text>

                    // 日志控制台
                    <Text text_style=TextStyle::Title>"实时日志"</Text>
                    <ScrollView max_height=300.0>
                        <Text text_style=TextStyle::Body>
                            {logs_snapshot}
                        </Text>
                    </ScrollView>
                </Column>
            }
        })
        .run();
}