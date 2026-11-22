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
    // 新增一个信号用于强制刷新
    let tick = signal(0);

    // 后台 Tokio 运行时
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(crate::core::background::run_background(cmd_rx, event_tx));
    });

    // 事件监听线程
    let logs_clone = logs.clone();
    let is_running_clone = is_running.clone();
    let tick_clone = tick.clone();
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
            // 每次有事件到来，就强行改变 tick 信号，这通常能触发 mitsuami 的重绘
            tick_clone.update(|t| *t += 1);
        }
    });

    // UI 主循环
    App::new()
        .window("NewAccl 加速器", Size::new(650.0, 500.0), move || {
            let cmd_tx_start = cmd_tx.clone();
            let cmd_tx_stop = cmd_tx.clone();
            
            // 依赖 tick 信号，强制重绘时重新读取字符串
            let _ = tick.get(); 
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
                        "提示：启动后程序会自动接管 Windows 系统代理。请确保程序有管理员权限（如果是修改系统注册表）。"
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