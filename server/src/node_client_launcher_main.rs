#![cfg_attr(windows, windows_subsystem = "windows")]

mod node_client_launcher;

fn main() {
    if let Err(error) = node_client_launcher::run() {
        eprintln!("一龙 PC 节点启动失败: {error:#}");
        std::process::exit(1);
    }
}
