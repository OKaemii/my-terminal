use anyhow::Result;
use myterm_settings::Settings;
use myterm_ui::app::TerminalApp;
use myterm_ui::plugin::Feature;

fn main() -> Result<()> {
    // 1. Load settings (falls back to defaults on error)
    let settings = Settings::load();

    // 2. Initialise feature flags from settings
    myterm_features::init_from_settings(&myterm_features::FeatureToggles {
        git_status:       settings.features.git_status,
        autosuggestions:  settings.features.autosuggestions,
        syntax_highlight: settings.features.syntax_highlight,
        jump:             settings.features.jump,
        predict_bar:      settings.features.predict_bar,
        fzf_files:        settings.features.fzf_files,
        command_not_found: settings.features.command_not_found,
    });

    // 3. Build feature registry
    let features: Vec<Box<dyn Feature>> = vec![
        // Feature crates will add implementations here in future phases.
        // For now the plugin hooks are wired but feature structs are not yet implemented.
    ];

    // 4. eframe native options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_inner_size([1200.0, 800.0])
            .with_title("myterm"),
        ..Default::default()
    };

    // 5. Load icon (best-effort)
    let options = load_icon(options);

    // 6. Launch
    eframe::run_native(
        "myterm",
        options,
        Box::new(move |cc| {
            let settings_clone = settings.clone();
            match TerminalApp::new(cc, settings_clone, features) {
                Ok(app) => Ok(Box::new(app) as Box<dyn eframe::App>),
                Err(e) => Err(e.into()),
            }
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}

fn load_icon(mut options: eframe::NativeOptions) -> eframe::NativeOptions {
    let icon_bytes = include_bytes!("../../legacy/assets/icon.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        options.viewport = options.viewport.with_icon(std::sync::Arc::new(
            egui::viewport::IconData {
                rgba: rgba.into_raw(),
                width: w,
                height: h,
            },
        ));
    }
    options
}
