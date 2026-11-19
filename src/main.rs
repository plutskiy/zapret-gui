#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod engine;
mod firewall;
mod net;
mod parallel;
mod settings;
mod sites;
mod sys;
mod telegram;
mod tester;
mod zapret;

fn main() -> eframe::Result {
    let no_elevate = std::env::args().any(|a| a == "--no-elevate");
    if !no_elevate && sys::needs_root() && !sys::is_elevated() && sys::relaunch_elevated() {
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1020.0, 720.0]).with_min_inner_size([760.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native("Zapret GUI", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
