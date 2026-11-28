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

    // 使用 signal 创建响应式变量
    let logs = signal(Vec::<String>::new());
    let is_running = signal(false);

    // 后台 Tokio 运行时
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(crate::core::background::run_background(cmd_rx, event_tx));
    });

    // 后台事件监听线程：更新 Signal
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

    // UI 主循环
    App::new()
        .window("NewAccl 加速器", Size::new(650.0, 500.0), move || {
            let cmd_tx_start = cmd_tx.clone();
            let cmd_tx_stop = cmd_tx.clone();

            view! {
                <Column gap=Spacing::Lg padding=Spacing::Xl>
                    <Row gap=Spacing::Md>
                        <Text text_style=TextStyle::Title>"NewAccl"</Text>
                        <Text text_style=TextStyle::Body>
                            // 修复类型错误：强制转换为 String
                            { move || if is_running.get() { "● 运行中 (已接管系统代理)".to_string() } else { "○ 已停止".to_string() } }
                        </Text>
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
                        "提示：启动后程序会自动接管 Windows 系统代理。"
                    </Text>

                    <Text text_style=TextStyle::Title>"实时日志"</Text>
                    <ScrollView max_height=300.0>
                        <Text text_style=TextStyle::Body>
                            // 每次日志变化时，这个闭包会被重新调用，从而刷新文本
                            { move || logs.get().join("\n") }
                        </Text>
                    </ScrollView>
                </Column>
            }
        })
        .run();
}