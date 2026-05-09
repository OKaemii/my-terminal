mod features;
mod pty;
mod terminal_app;
mod vte_proc;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("myterm")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([480.0, 300.0])
            .with_app_id("dev.myterm.myterm")
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "myterm",
        options,
        Box::new(|cc| Ok(Box::new(terminal_app::TerminalApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes)
        .expect("embedded icon.png is valid")
        .to_rgba8();
    let (width, height) = img.dimensions();
    egui::IconData { rgba: img.into_raw(), width, height }
}
