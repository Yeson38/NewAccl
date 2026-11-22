mod core;
mod ui;

fn main() -> anyhow::Result<()> {
    // UI 框架会在内部启动线程和后台通信
    ui::run_ui();
    Ok(())
}
