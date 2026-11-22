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

    let logs = signal(Vec::<String>::new());
    let is_running = signal(false);

    // 后台 Tokio 运行时
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(crate::core::background::run_background(cmd_rx, event_tx));
    });

    // UI 主循环
    App::new()
        .window("NewAccl 加速器", Size::new(650.0, 500.0), move || {
            let cmd_tx_start = cmd_tx.clone();
            let cmd_tx_stop = cmd_tx.clone();
            
            // 关键修复：在 UI 渲染循环中同步接收事件
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

            let current_logs = logs.get().join("\n");
            let status_text = if is_running.get() { "● 运行中 (已接管系统代理)" } else { "○ 已停止" };

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
                        "提示：启动后程序会自动接管 Windows 系统代理。如果界面状态未刷新，请在窗口内移动一下鼠标或点击一下窗口。"
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