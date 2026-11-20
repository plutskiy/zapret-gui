use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub static QUITTING: AtomicBool = AtomicBool::new(false);

pub fn quitting() -> bool {
    QUITTING.load(Ordering::Relaxed)
}

pub fn hide(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
}

fn show(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

pub fn create(ctx: &egui::Context) -> Option<TrayIcon> {
    let img = image::load_from_memory(include_bytes!("../installer/zapret.ico"))
        .ok()?
        .into_rgba8();
    let (w, h) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), w, h).ok()?;

    let open = MenuItem::with_id("open", "Открыть", true, None);
    let quit = MenuItem::with_id("quit", "Выход", true, None);
    let menu = Menu::new();
    menu.append_items(&[&open, &quit]).ok()?;

    let c = ctx.clone();
    TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = e
        {
            show(&c);
        }
    }));
    let c = ctx.clone();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| match e.id.0.as_str() {
        "open" => show(&c),
        "quit" => {
            QUITTING.store(true, Ordering::Relaxed);
            c.send_viewport_cmd(egui::ViewportCommand::Close);
            c.request_repaint();
        }
        _ => {}
    }));

    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("Zapret GUI")
        .with_icon(icon)
        .build()
        .ok()
}
