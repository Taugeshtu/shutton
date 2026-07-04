use gtk4::{self as gtk, glib, prelude::*, Application, ApplicationWindow, Box, Button, CheckButton, Entry, Label, Orientation, ScrolledWindow, TextView};
use std::sync::{Arc, Mutex};

enum LogEvent {
    Line(String),
    Finished,
}

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

        let window = ApplicationWindow::builder()
            .application(app).title("shutton")
            .default_width(900).default_height(100).build();
        
        let vbox = Box::new(Orientation::Vertical, 6);
        vbox.set_margin_top(8); vbox.set_margin_bottom(8);
        vbox.set_margin_start(8); vbox.set_margin_end(8);

        // Top row: Entry | Quit Toggle | RUN
        let hbox_top = Box::new(Orientation::Horizontal, 8);
        let entry = Entry::builder().hexpand(true).placeholder_text("Enter command...").build();
        
        let quit_toggle = CheckButton::new();
        quit_toggle.set_active(true);
        quit_toggle.set_tooltip_text(Some("Quit on done"));
        
        let run_btn = Button::with_label("RUN");
        hbox_top.append(&entry);
        hbox_top.append(&quit_toggle);
        hbox_top.append(&run_btn);

        // Separator
        let sep = gtk::Separator::new(Orientation::Horizontal);

        // Bottom row: Log label on left, spacer in middle, icon buttons on right
        let hbox_bottom = Box::new(Orientation::Horizontal, 8);
        
        let log_label = Label::new(Some("Log actions:"));
        log_label.set_opacity(0.6);
        
        let spacer = Box::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        
        let buttons_box = Box::new(Orientation::Horizontal, 6);
        
        let v_btn = Button::from_icon_name("pan-down-symbolic");
        v_btn.set_tooltip_text(Some("Toggle log view"));
        
        let c_btn = Button::from_icon_name("edit-copy-symbolic");
        c_btn.set_tooltip_text(Some("Copy log to clipboard"));
        
        let o_btn = Button::from_icon_name("document-save-symbolic");
        o_btn.set_tooltip_text(Some("Save log to output.log"));
        
        buttons_box.append(&v_btn);
        buttons_box.append(&c_btn);
        buttons_box.append(&o_btn);

        hbox_bottom.append(&log_label);
        hbox_bottom.append(&spacer);
        hbox_bottom.append(&buttons_box);

        // Log scroll (hidden by default)
        let log_scroll = ScrolledWindow::builder()
            .min_content_height(150)
            .vexpand(true)
            .visible(false)
            .build();
        let log_view = TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::Word)
            .build();
        log_scroll.set_child(Some(&log_view));

        vbox.append(&hbox_top);
        vbox.append(&sep);
        vbox.append(&hbox_bottom);
        vbox.append(&log_scroll);

        // Shared log store
        let log_buffer = Arc::new(Mutex::new(String::new()));

        // Button [v] toggles visibility and window height
        let log_scroll_clone = log_scroll.clone();
        let window_clone = window.clone();
        v_btn.connect_clicked(move |_| {
            let is_visible = !log_scroll_clone.is_visible();
            log_scroll_clone.set_visible(is_visible);
            if is_visible {
                window_clone.set_default_size(900, 300);
            } else {
                window_clone.set_default_size(900, 100);
            }
        });

        // Button [c] copies log to clipboard
        let log_buffer_clone = log_buffer.clone();
        c_btn.connect_clicked(move |_| {
            let text = log_buffer_clone.lock().unwrap().clone();
            if let Some(display) = gtk::gdk::Display::default() {
                display.clipboard().set_text(&text);
            }
        });

        // Button [o] writes output.log next to binary
        let log_buffer_clone2 = log_buffer.clone();
        o_btn.connect_clicked(move |_| {
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(parent) = exe_path.parent() {
                    let log_path = parent.join("output.log");
                    let text = log_buffer_clone2.lock().unwrap().clone();
                    let _ = std::fs::write(log_path, text);
                }
            }
        });

        // Runner closure
        let entry_clone = entry.clone();
        let run_btn_clone = run_btn.clone();
        let log_view_clone = log_view.clone();
        let log_buffer_receiver = log_buffer.clone();
        let quit_toggle_clone = quit_toggle.clone();
        let app_clone = app.clone();

        let run_cmd = move || {
            let cmd = entry_clone.text().to_string();
            if cmd.is_empty() { return; }
            
            // Clear UI log
            log_view_clone.buffer().set_text("");
            log_buffer_receiver.lock().unwrap().clear();
            run_btn_clone.set_sensitive(false);

            // Create standard channel
            let (sender, receiver) = std::sync::mpsc::channel::<LogEvent>();

            // Spawn worker thread
            std::thread::spawn(move || {
                use std::io::{BufRead, BufReader};
                use std::process::{Command, Stdio};
                
                let child = Command::new("sh")
                    .arg("-c")
                    .arg(format!("{} 2>&1", cmd))
                    .stdout(Stdio::piped())
                    .spawn();

                match child {
                    Ok(mut child) => {
                        if let Some(stdout) = child.stdout.take() {
                            let reader = BufReader::new(stdout);
                            for line in reader.lines() {
                                if let Ok(mut l) = line {
                                    l.push('\n');
                                    let _ = sender.send(LogEvent::Line(l));
                                }
                            }
                        }
                        let _ = child.wait();
                        let _ = sender.send(LogEvent::Finished);
                    }
                    Err(e) => {
                        let _ = sender.send(LogEvent::Line(format!("Error: {}\n", e)));
                        let _ = sender.send(LogEvent::Finished);
                    }
                }
            });

            // Start GLib timeout to poll receiver
            let log_view_poll = log_view_clone.clone();
            let log_buffer_poll = log_buffer_receiver.clone();
            let run_btn_poll = run_btn_clone.clone();
            let quit_toggle_poll = quit_toggle_clone.clone();
            let app_poll = app_clone.clone();

            glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
                while let Ok(event) = receiver.try_recv() {
                    match event {
                        LogEvent::Line(line) => {
                            let buf = log_view_poll.buffer();
                            let mut end = buf.end_iter();
                            buf.insert(&mut end, &line);
                            
                            let mut log = log_buffer_poll.lock().unwrap();
                            log.push_str(&line);
                        }
                        LogEvent::Finished => {
                            run_btn_poll.set_sensitive(true);
                            if quit_toggle_poll.is_active() {
                                app_poll.quit();
                            }
                            return glib::ControlFlow::Break;
                        }
                    }
                }
                glib::ControlFlow::Continue
            });
        };

        let run_cmd_btn = run_cmd.clone();
        run_btn.connect_clicked(move |_| run_cmd_btn());

        let key_ctrl = gtk::EventControllerKey::new();
        key_ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);
        let app_clone2 = app.clone();
        let run_cmd_key = run_cmd;
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            match key {
                gtk::gdk::Key::Escape => {
                    app_clone2.quit();
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

        window.set_child(Some(&vbox));
        window.present();
    });
    app.run();
}
