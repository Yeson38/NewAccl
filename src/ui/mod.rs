use mitsuami::prelude::*;
use crossbeam_channel::unbounded;
use std::thread;

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

    // 正确方式：使用 signal() 函数创建响应式状态
    let logs = signal(Vec::<String>::new());
    let is_running = signal(false);

    // 后台运行逻辑
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(crate::core::background::run_background(cmd_rx, event_tx));
    });

    // UI 主循环
    App::new()
        .window("NewAccl 加速器", Size::new(650.0, 500.0), move || {
            // 轮询处理通道事件，并更新 Signal 触发 UI 重绘
            while let Ok(event) = event_rx.try_recv() {
                match event {
                    Event::Log(msg) => {
                        logs.update(|l| {
                            l.push(format!("> {}", msg));
                            if l.len() > 100 { l.remove(0); }
                        });
                    }
                    Event::Status(status) => {
                        is_running.set(status);
                    }
                }
            }

            let cmd_tx_start = cmd_tx.clone();
            let cmd_tx_stop = cmd_tx.clone();
            // 通过 get() 读取 Signal 的当前值
            let current_logs = logs.get().join("\n");
            let status_text = if is_running.get() { "● 运行中" } else { "○ 已停止" };

            view! {
                <Column gap=Spacing::Lg padding=Spacing::Xl>
                    <Row gap=Spacing::Md>
                        <Text text_style=TextStyle::Title>"NewAccl"</Text>
                        <Text text_style=TextStyle::Body>{status_text}</Text>
                    </Row>

                    <Row gap=Spacing::Md>
                        <Button @click=move || {
                            let _ = cmd_tx_start.send(Command::StartProxy);
                        }>"启动代理"</Button>
                        
                        <Button @click=move || {
                            let _ = cmd_tx_stop.send(Command::StopProxy);
                        }>"停止代理"</Button>
                    </Row>

                    <Text text_style=TextStyle::Caption>
                        "提示：请在系统设置中将 HTTP 代理设置为 127.0.0.1:8443。如果无法启动，请先关闭残留的黑框进程。"
                    </Text>

                    <Text text_style=TextStyle::Title>"实时日志"</Text>
                    <ScrollView max_height=300.0>
                        <Text text_style=TextStyle::Body>
                            {current_logs}
                        </Text>
                    </ScrollView>
                </Column>
            }
        })
        .run();
}