use gtk4::{self as gtk, glib, prelude::*, Application, ApplicationWindow, Box, Button, Entry, Orientation};

fn main() {
    let app = Application::builder().application_id("games.tau.shutton").build();
    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app).title("shutton")
            .default_width(900).default_height(60).build();
        
        let hbox = Box::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(8); hbox.set_margin_bottom(8);
        hbox.set_margin_start(8); hbox.set_margin_end(8);
        
        let entry = Entry::builder().hexpand(true).placeholder_text("Enter command...").build();
        let run_btn = Button::with_label("RUN");
        
        let run_cmd = {
            let entry = entry.clone();
            move || {
                let cmd = entry.text().to_string();
                if !cmd.is_empty() {
                    println!("Running: {}", cmd);
                    let _ = std::process::Command::new("sh").arg("-c").arg(cmd).status();
                }
            }
        };

        let run_cmd_btn = run_cmd.clone();
        run_btn.connect_clicked(move |_| run_cmd_btn());

        let key_ctrl = gtk::EventControllerKey::new();
        key_ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);
        let app_clone = app.clone();
        let run_cmd_key = run_cmd;
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            println!("Key pressed: {:?}", key);
            match key {
                gtk::gdk::Key::Escape => {
                    app_clone.quit();
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                    run_cmd_key();
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        window.add_controller(key_ctrl);

        hbox.append(&entry);
        hbox.append(&run_btn);
        window.set_child(Some(&hbox));
        window.present();
    });
    app.run();
}
