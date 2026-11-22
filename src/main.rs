#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod ui;

fn main() -> anyhow::Result<()> {
    ui::run_ui();
    Ok(())
}