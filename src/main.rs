use gtk4::{self as gtk, prelude::*, Application};

mod ui;
mod core;
mod storage;

fn main() {
    let app = Application::builder().application_id("games.tau.shutton").build();
    app.connect_activate(|app| {
        // Load CSS for transparent inputs/logs and slim buttons
        let provider = gtk::CssProvider::new();
        provider.load_from_string(
            "entry, entry > text, scrolledwindow, textview, textview text { background: transparent; }
             button { padding: 4px 8px; min-height: 24px; min-width: 24px; }
             label { font-size: 13px; }"
        );
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        ui::activate(app);
    });
    app.run();
}
