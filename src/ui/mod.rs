use mitsuami::prelude::*;
use crossbeam_channel::unbounded;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub enum Command {
    StartProxy,
    StopProxy,
}

pub enum Event {
    Log(String),
    Status(bool),
    Tick, // 强制刷新信号
}

pub fn run_ui() {
    let (cmd_tx, cmd_rx) = unbounded::<Command>();
    let (event_tx, event_rx) = unbounded::<Event>();

    // 使用 Arc<Mutex> 来安全地共享状态
    let logs = Arc::new(Mutex::new(Vec::<String>::new()));
    let is_running = Arc::new(Mutex::new(false));

    // 1. 启动后台代理任务
    let event_tx_clone = event_tx.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(crate::core::background::run_background(cmd_rx, event_tx_clone));
    });

    // 2. 启动一个定时器，每 500 毫秒发送一个 Tick 唤醒 UI
    let event_tx_timer = event_tx.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(500));
            let _ = event_tx_timer.send(Event::Tick);
        }
    });

    // 3. UI 主循环
    App::new()
        .window("NewAccl 加速器", Size::new(650.0, 500.0), move || {
            let cmd_tx_start = cmd_tx.clone();
            let cmd_tx_stop = cmd_tx.clone();
            
            // 在每次渲染时，读取所有积压的事件并更新本地状态
            while let Ok(event) = event_rx.try_recv() {
                match event {
                    Event::Log(msg) => {
                        let mut l = logs.lock().unwrap();
                        l.push(format!("> {}", msg));
                        if l.len() > 100 { l.remove(0); }
                    }
                    Event::Status(status) => {
                        let mut r = is_running.lock().unwrap();
                        *r = status;
                    }
                    Event::Tick => {
                        // 收到定时器信号，啥也不干，单纯让闭包重新执行
                    }
                }
            }

            let current_logs = logs.lock().unwrap().join("\n");
            let running_state = *is_running.lock().unwrap();
            let status_text = if running_state { "● 运行中 (已接管系统代理)" } else { "○ 已停止" };

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
                        "提示：启动后程序会自动接管 Windows 系统代理。界面每 0.5 秒自动刷新一次日志。"
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