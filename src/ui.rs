use gtk4::{self as gtk, glib, prelude::*, Application, ApplicationWindow, Box, Button, CheckButton, Entry, Label, Orientation, ScrolledWindow, TextView};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::runner::{execute, LogEvent};

pub fn activate(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app).title("shutton")
        .default_width(900).default_height(100).build();
    
    let vbox = Box::new(Orientation::Vertical, 6);
    vbox.set_margin_top(8); vbox.set_margin_bottom(8);
    vbox.set_margin_start(8); vbox.set_margin_end(8);

    // Main layout: left side for inputs, right side for controls
    let hbox_main = Box::new(Orientation::Horizontal, 8);

    let vbox_left = Box::new(Orientation::Vertical, 6);
    vbox_left.set_hexpand(true);

    let entry = Entry::builder().hexpand(true).placeholder_text("Enter command...").build();
    vbox_left.append(&entry);

    // Container for dynamic variable rows
    let vbox_vars = Box::new(Orientation::Vertical, 6);
    vbox_left.append(&vbox_vars);

    // Right column: houses checkbox and run button
    let vbox_right = Box::new(Orientation::Vertical, 6);
    
    let quit_toggle = CheckButton::with_label("quit");
    quit_toggle.set_active(true);
    quit_toggle.set_tooltip_text(Some("Quit on done"));
    quit_toggle.set_valign(gtk::Align::Center);
    
    let run_btn = Button::from_icon_name("go-next-symbolic");
    run_btn.set_tooltip_text(Some("Run command"));
    run_btn.set_vexpand(true);
    run_btn.set_valign(gtk::Align::Fill);
    run_btn.set_halign(gtk::Align::Fill);

    vbox_right.append(&quit_toggle);
    vbox_right.append(&run_btn);

    hbox_main.append(&vbox_left);
    hbox_main.append(&vbox_right);

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

    vbox.append(&hbox_main);
    vbox.append(&sep);
    vbox.append(&hbox_bottom);
    vbox.append(&log_scroll);

    // Shared state
    let log_buffer = Arc::new(Mutex::new(String::new()));
    let var_values = Arc::new(Mutex::new(HashMap::<String, String>::new()));
    let current_vars = Arc::new(Mutex::new(Vec::<String>::new()));

    // Window resize helper
    let resize_window = {
        let window = window.clone();
        let log_scroll = log_scroll.clone();
        move |vars_count: usize| {
            let base_height = if log_scroll.is_visible() { 300 } else { 100 };
            let target_height = base_height + (vars_count * 38) as i32;
            window.set_default_size(900, target_height);
        }
    };

    // Parse variables and rebuild rows
    let update_vars = {
        let vbox_vars = vbox_vars.clone();
        let entry = entry.clone();
        let var_values = var_values.clone();
        let current_vars = current_vars.clone();
        let resize_window = resize_window.clone();
        
        move || {
            let text = entry.text().to_string();
            
            // Parse unique variables in order
            let mut new_vars = Vec::new();
            let mut chars = text.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '%' {
                    let mut name = String::new();
                    while let Some(&next_c) = chars.peek() {
                        if next_c.is_whitespace() {
                            break;
                        }
                        name.push(chars.next().unwrap());
                    }
                    if !name.is_empty() && !new_vars.contains(&name) {
                        new_vars.push(name);
                    }
                }
            }

            // Rebuild only if variables list changed
            let vars_changed = {
                let mut current = current_vars.lock().unwrap();
                if *current == new_vars {
                    false
                } else {
                    *current = new_vars.clone();
                    true
                }
            };

            if !vars_changed {
                return;
            }

            // Clear old dynamic widgets in vbox_vars
            while let Some(child) = vbox_vars.first_child() {
                vbox_vars.remove(&child);
            }

            // Create and attach new rows
            for var in new_vars.iter() {
                let row_box = Box::new(Orientation::Horizontal, 8);

                let label = Label::new(Some(var));
                label.set_width_chars(12);
                label.set_max_width_chars(12);
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                label.set_halign(gtk::Align::End);
                label.set_opacity(0.8);

                let val = var_values.lock().unwrap().get(var).cloned().unwrap_or_default();
                let var_entry = Entry::builder()
                    .hexpand(true)
                    .text(&val)
                    .placeholder_text(&format!("Value for %{}...", var))
                    .build();

                let var_values_clone = var_values.clone();
                let var_clone = var.clone();
                var_entry.connect_changed(move |e| {
                    var_values_clone.lock().unwrap().insert(var_clone.clone(), e.text().to_string());
                });

                row_box.append(&label);
                row_box.append(&var_entry);
                vbox_vars.append(&row_box);
            }

            resize_window(new_vars.len());
        }
    };

    let update_vars_clone = update_vars.clone();
    entry.connect_changed(move |_| {
        update_vars_clone();
    });

    // Button [v] toggles visibility and resizes window
    let log_scroll_clone = log_scroll.clone();
    let resize_window_clone = resize_window.clone();
    let current_vars_clone = current_vars.clone();
    v_btn.connect_clicked(move |_| {
        let is_visible = !log_scroll_clone.is_visible();
        log_scroll_clone.set_visible(is_visible);
        let count = current_vars_clone.lock().unwrap().len();
        resize_window_clone(count);
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
        let main_cmd = entry_clone.text().to_string();
        if main_cmd.is_empty() { return; }
        
        // Substitute variables
        let mut cmd = main_cmd.clone();
        let values = var_values.lock().unwrap();
        for (var, val) in values.iter() {
            cmd = cmd.replace(&format!("%{}", var), val);
        }

        // Clear UI log
        log_view_clone.buffer().set_text("");
        log_buffer_receiver.lock().unwrap().clear();
        run_btn_clone.set_sensitive(false);

        // Create standard channel
        let (sender, receiver) = std::sync::mpsc::channel::<LogEvent>();

        // Spawn worker thread via runner
        execute(cmd, sender);

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
}
