use mitsuami::prelude::*;
use crossbeam_channel::{Receiver, Sender};
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
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<Command>();
    // 用于 后台 -> UI 的事件通道
    let (event_tx, event_rx) = crossbeam_channel::unbounded::<Event>();

    // 启动后台 Tokio 运行时
    let event_tx_clone = event_tx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(crate::core::background::run_background(cmd_rx, event_tx_clone));
    });

    // 启动 mitsuami App
    App::new()
        .title("NewAccl 加速器")
        .size(600, 400)
        .view(move || {
            // 从通道接收日志 (非阻塞)
            let mut logs = Vec::new();
            while let Ok(event) = event_rx.try_recv() {
                if let Event::Log(msg) = event {
                    logs.push(msg);
                }
            }

            // 用 Arc<Mutex> 维持状态 (简易做法)
            let cmd_tx = cmd_tx.clone();

            VStack::new()
                .padding(20)
                .spacing(10)
                .children((
                    HStack::new()
                        .spacing(10)
                        .children((
                            Button::new("启动代理")
                                .on_click(move || {
                                    let _ = cmd_tx.send(Command::StartProxy);
                                }),
                            Button::new("停止代理")
                                .on_click({
                                    let cmd_tx = cmd_tx.clone();
                                    move || {
                                        let _ = cmd_tx.send(Command::StopProxy);
                                    }
                                }),
                        )),
                    // 日志区域
                    ScrollView::new(
                        Text::new(logs.join("\n"))
                            .font_size(12)
                            .monospace()
                    )
                    .frame_max_height(250),
                ))
        })
        .run();
}
