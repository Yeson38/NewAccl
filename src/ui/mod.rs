use mitsuami::prelude::*;
use crossbeam_channel::unbounded;
use std::thread;

// 定义 UI 发给后台的命令
pub enum Command {
    StartProxy,
    StopProxy,
}

// 定义后台发给 UI 的事件
pub enum Event {
    Log(String),
    Status(bool), // true = 运行中, false = 已停止
}

pub fn run_ui() {
    // 用于 UI -> 后台 的命令通道
    let (cmd_tx, cmd_rx) = unbounded::<Command>();
    // 用于 后台 -> UI 的事件通道
    let (event_tx, event_rx) = unbounded::<Event>();

    // 启动后台 Tokio 运行时
    let event_tx_clone = event_tx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(crate::core::background::run_background(cmd_rx, event_tx_clone));
    });

    // 启动 mitsuami App
    App::new()
        .window("NewAccl 加速器", Size::new(600.0, 400.0), move || {
            // 从通道接收日志 (非阻塞)
            let mut logs = Vec::new();
            while let Ok(event) = event_rx.try_recv() {
                if let Event::Log(msg) = event {
                    logs.push(msg);
                }
            }

            // 关键修复：为每个按钮单独克隆一个发送端
            let cmd_tx_start = cmd_tx.clone();
            let cmd_tx_stop = cmd_tx.clone();
            
            view! {
                <Column gap=Spacing::Md padding=Spacing::Lg>
                    <Row gap=Spacing::Md>
                        <Button @click=move || {
                            let _ = cmd_tx_start.send(Command::StartProxy);
                        }>"启动代理"</Button>
                        <Button @click=move || {
                            let _ = cmd_tx_stop.send(Command::StopProxy);
                        }>"停止代理"</Button>
                    </Row>
                    <ScrollView max_height=250.0>
                        <Text text_style=TextStyle::Body>
                            {logs.join("\n")}
                        </Text>
                    </ScrollView>
                </Column>
            }
        })
        .run();
}