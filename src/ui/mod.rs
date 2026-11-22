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

    // 创建响应式状态
    let logs = signal(Vec::<String>::new());
    let is_running = signal(false);

    // 1. 后台 Tokio 运行时 (处理代理)
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(crate::core::background::run_background(cmd_rx, event_tx));
    });

    // 2. 后台事件监听线程 (把通道里的消息转成 Signal，触发 UI 重绘)
    let logs_clone = logs.clone();
    let is_running_clone = is_running.clone();
    thread::spawn(move || {
        while let Ok(event) = event_rx.recv() {
            match event {
                Event::Log(msg) => {
                    logs_clone.update(|l| {
                        l.push(format!("> {}", msg));
                        if l.len() > 100 { l.remove(0); }
                    });
                }
                Event::Status(status) => {
                    is_running_clone.set(status);
                }
            }
        }
    });

    // 3. UI 主循环
    App::new()
        .window("NewAccl 加速器", Size::new(650.0, 500.0), move || {
            let cmd_tx_start = cmd_tx.clone();
            let cmd_tx_stop = cmd_tx.clone();
            
            // 读取 Signal 的当前值
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
                        "提示：请在系统设置中将 HTTP 代理设置为 127.0.0.1:8443。"
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