use gtk4::{self as gtk, glib, prelude::*, Application, ApplicationWindow, Box, Button, CheckButton, Entry, Label, Orientation, ScrolledWindow, TextView, ToggleButton};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::core::{execute, parse_vars, LogEvent};

fn build_var_rows(
    vbox_vars: &Box,
    vars: &[String],
    var_values: &Arc<Mutex<HashMap<String, String>>>,
) {
    // Clear old dynamic widgets in vbox_vars
    while let Some(child) = vbox_vars.first_child() {
        vbox_vars.remove(&child);
    }

    // Create and attach new rows
    for var in vars {
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
        var_entry.add_css_class("monospace");

        let var_values_clone = var_values.clone();
        let var_clone = var.clone();
        var_entry.connect_changed(move |e| {
            var_values_clone.lock().unwrap().insert(var_clone.clone(), e.text().to_string());
        });

        row_box.append(&label);
        row_box.append(&var_entry);
        vbox_vars.append(&row_box);
    }
}

fn grab_initial_focus(entry: &Entry, vbox_vars: &Box) {
    let mut focus_grabbed = false;
    if let Some(first_child) = vbox_vars.first_child() {
        if let Some(row_box) = first_child.downcast_ref::<gtk::Box>() {
            if let Some(entry_widget) = row_box.last_child() {
                if let Some(var_entry) = entry_widget.downcast_ref::<gtk::Entry>() {
                    var_entry.grab_focus();
                    focus_grabbed = true;
                }
            }
        }
    }
    if !focus_grabbed {
        entry.grab_focus();
    }
}

pub fn activate(app: &Application) {
    // Load initial config from binary tail
    let config_opt = crate::storage::read_config();
    let initial_cmd = if let Some(ref cfg) = config_opt {
        cfg.main_cmd.clone()
    } else {
        String::new()
    };
    let initial_autoquit = if let Some(ref cfg) = config_opt {
        cfg.autoquit
    } else {
        true
    };
    let initial_width = if let Some(ref cfg) = config_opt {
        cfg.width
    } else {
        900
    };
    let initial_description = if let Some(ref cfg) = config_opt {
        cfg.description.clone()
    } else {
        String::new()
    };
    let initial_active_fold = if let Some(ref cfg) = config_opt {
        cfg.active_fold
    } else {
        0
    };

    let window = ApplicationWindow::builder()
        .application(app).title("shutton")
        .default_width(initial_width).default_height(100).build();
    
    let vbox = Box::new(Orientation::Vertical, 6);
    vbox.set_margin_top(8); vbox.set_margin_bottom(8);
    vbox.set_margin_start(8); vbox.set_margin_end(8);

    // Main layout: left side for inputs, right side for controls
    let hbox_main = Box::new(Orientation::Horizontal, 8);

    let vbox_left = Box::new(Orientation::Vertical, 6);
    vbox_left.set_hexpand(true);

    let entry = Entry::builder().hexpand(true).placeholder_text("Enter command...").build();
    entry.add_css_class("monospace");
    vbox_left.append(&entry);

    // Container for dynamic variable rows
    let vbox_vars = Box::new(Orientation::Vertical, 6);
    vbox_left.append(&vbox_vars);

    // Right column: houses checkbox and run button
    let vbox_right = Box::new(Orientation::Vertical, 6);
    
    let quit_toggle = CheckButton::with_label("quit");
    quit_toggle.set_active(initial_autoquit);
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

    // Bottom row: Description button on left, spacer in middle, log actions on right
    let hbox_bottom = Box::new(Orientation::Horizontal, 8);
    
    let desc_btn = ToggleButton::with_label("Description");
    desc_btn.set_tooltip_text(Some("Toggle description view"));
    
    let spacer = Box::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    
    let buttons_box = Box::new(Orientation::Horizontal, 6);
    
    let log_label = Label::new(Some("log actions:"));
    log_label.set_opacity(0.6);
    
    let v_btn = ToggleButton::builder().icon_name("pan-down-symbolic").build();
    v_btn.set_tooltip_text(Some("Toggle log view"));
    
    let c_btn = Button::from_icon_name("edit-copy-symbolic");
    c_btn.set_tooltip_text(Some("Copy log to clipboard"));
    
    let exe_path_opt = std::env::current_exe().ok();
    let bin_name = exe_path_opt.as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("shutton");
    let log_filename = format!("{}.log", bin_name);

    let o_btn = Button::from_icon_name("document-save-symbolic");
    o_btn.set_tooltip_text(Some(&format!("Save log to {}", log_filename)));
    
    buttons_box.append(&log_label);
    buttons_box.append(&v_btn);
    buttons_box.append(&c_btn);
    buttons_box.append(&o_btn);

    hbox_bottom.append(&desc_btn);
    hbox_bottom.append(&spacer);
    hbox_bottom.append(&buttons_box);

    // Description scroll (hidden by default)
    let desc_scroll = ScrolledWindow::builder()
        .min_content_height(150)
        .vexpand(true)
        .visible(false)
        .build();
    let desc_view = TextView::builder()
        .wrap_mode(gtk::WrapMode::Word)
        .build();
    desc_view.buffer().set_text(&initial_description);
    desc_scroll.set_child(Some(&desc_view));

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
    log_view.add_css_class("monospace");
    log_scroll.set_child(Some(&log_view));

    vbox.append(&hbox_main);
    vbox.append(&sep);
    vbox.append(&hbox_bottom);
    vbox.append(&log_scroll);
    vbox.append(&desc_scroll);

    // Shared state
    let log_buffer = Arc::new(Mutex::new(String::new()));
    let var_values = Arc::new(Mutex::new(HashMap::<String, String>::new()));
    
    // Seed initial var values from loaded config
    if let Some(ref cfg) = config_opt {
        let parsed = parse_vars(&cfg.main_cmd);
        let mut vals = var_values.lock().unwrap();
        for (i, var) in parsed.iter().enumerate() {
            if let Some(val) = cfg.var_values.get(i) {
                vals.insert(var.clone(), val.clone());
            }
        }
    }

    let current_vars = Arc::new(Mutex::new(Vec::<String>::new()));

    // Window resize helper - preserves active user-resized width
    let resize_window = {
        let window = window.clone();
        let log_scroll = log_scroll.clone();
        let desc_scroll = desc_scroll.clone();
        move |vars_count: usize| {
            let base_height = if log_scroll.is_visible() || desc_scroll.is_visible() { 300 } else { 100 };
            let target_height = base_height + (vars_count * 38) as i32;
            let current_width = if window.is_realized() {
                window.width()
            } else {
                initial_width
            };
            window.set_default_size(current_width, target_height);
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
            let new_vars = parse_vars(&text);

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

            build_var_rows(&vbox_vars, &new_vars, &var_values);
            resize_window(new_vars.len());
        }
    };

    let update_vars_clone = update_vars.clone();
    entry.connect_changed(move |_| {
        update_vars_clone();
    });

    // Populate main entry which triggers the rows to render
    entry.set_text(&initial_cmd);

    // Button [v] toggles visibility and resizes window
    let log_scroll_clone = log_scroll.clone();
    let desc_scroll_clone = desc_scroll.clone();
    let desc_btn_clone = desc_btn.clone();
    let resize_window_clone = resize_window.clone();
    let current_vars_clone = current_vars.clone();
    v_btn.connect_toggled(move |btn| {
        let active = btn.is_active();
        if active {
            desc_btn_clone.set_active(false);
            desc_scroll_clone.set_visible(false);
        }
        log_scroll_clone.set_visible(active);
        let count = current_vars_clone.lock().unwrap().len();
        resize_window_clone(count);
    });

    // Description button toggles visibility and resizes window
    let log_scroll_clone2 = log_scroll.clone();
    let desc_scroll_clone2 = desc_scroll.clone();
    let v_btn_clone = v_btn.clone();
    let resize_window_clone2 = resize_window.clone();
    let current_vars_clone2 = current_vars.clone();
    desc_btn.connect_toggled(move |btn| {
        let active = btn.is_active();
        if active {
            v_btn_clone.set_active(false);
            log_scroll_clone2.set_visible(false);
        }
        desc_scroll_clone2.set_visible(active);
        let count = current_vars_clone2.lock().unwrap().len();
        resize_window_clone2(count);
    });

    // Set initial active states (triggers toggled event handlers)
    v_btn.set_active(initial_active_fold == 1);
    desc_btn.set_active(initial_active_fold == 2);

    // Button [c] copies log to clipboard
    let log_buffer_clone = log_buffer.clone();
    c_btn.connect_clicked(move |_| {
        let text = log_buffer_clone.lock().unwrap().clone();
        if let Some(display) = gtk::gdk::Display::default() {
            display.clipboard().set_text(&text);
        }
    });

    // Button [o] writes log next to binary
    let log_buffer_clone2 = log_buffer.clone();
    let log_filename_clone = log_filename.clone();
    o_btn.connect_clicked(move |_| {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let log_path = parent.join(&log_filename_clone);
                let text = log_buffer_clone2.lock().unwrap().clone();
                let _ = std::fs::write(log_path, text);
            }
        }
    });

    // Compile config helper
    let compile_config = {
        let entry = entry.clone();
        let quit_toggle = quit_toggle.clone();
        let window = window.clone();
        let log_scroll = log_scroll.clone();
        let desc_scroll = desc_scroll.clone();
        let desc_view = desc_view.clone();
        let var_values = var_values.clone();
        
        move || {
            let main_cmd = entry.text().to_string();
            let autoquit = quit_toggle.is_active();
            let width = window.width();

            let active_fold = if log_scroll.is_visible() {
                1
            } else if desc_scroll.is_visible() {
                2
            } else {
                0
            };
            let description = {
                let buffer = desc_view.buffer();
                let start = buffer.start_iter();
                let end = buffer.end_iter();
                buffer.text(&start, &end, false).to_string()
            };

            let parsed_vars = parse_vars(&main_cmd);
            let mut var_vals = Vec::new();
            {
                let values = var_values.lock().unwrap();
                for var in &parsed_vars {
                    let val = values.get(var).cloned().unwrap_or_default();
                    var_vals.push(val);
                }
            }
            crate::storage::Config {
                autoquit,
                width,
                main_cmd,
                var_values: var_vals,
                description,
                active_fold,
            }
        }
    };

    // Runner closure
    let run_btn_clone = run_btn.clone();
    let log_view_clone = log_view.clone();
    let log_buffer_receiver = log_buffer.clone();
    let quit_toggle_clone = quit_toggle.clone();
    let app_clone = app.clone();
    let compile_config_run = compile_config.clone();

    let run_cmd = move || {
        let config = compile_config_run();
        if config.main_cmd.is_empty() { return; }
        
        if let Err(e) = crate::storage::patch_config(&config) {
            eprintln!("Error saving config to binary: {}", e);
        }

        // 2. Clear UI log
        log_view_clone.buffer().set_text("");
        log_buffer_receiver.lock().unwrap().clear();
        run_btn_clone.set_sensitive(false);

        // Create standard channel
        let (sender, receiver) = std::sync::mpsc::channel::<LogEvent>();

        // 3. Delegate execution & binary saving to core module
        execute(config, sender);

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
    let compile_config_key = compile_config.clone();
    let entry_focus = entry.clone();
    let run_btn_focus = run_btn.clone();
    let vbox_vars_focus = vbox_vars.clone();
    let window_focus = window.clone();

    key_ctrl.connect_key_pressed(move |_, key, _, state| {
        let is_ctrl = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
        if is_ctrl && (key == gtk::gdk::Key::s || key == gtk::gdk::Key::S) {
            let config = compile_config_key();
            if let Err(e) = crate::storage::patch_config(&config) {
                eprintln!("Error saving config to binary: {}", e);
            }
            return glib::Propagation::Stop;
        }

        match key {
            gtk::gdk::Key::Escape => {
                app_clone2.quit();
                glib::Propagation::Stop
            }
            gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                let is_input_focused = if let Some(focused) = GtkWindowExt::focus(&window_focus) {
                    focused == *entry_focus.upcast_ref::<gtk4::Widget>()
                        || focused.is_ancestor(&entry_focus)
                        || focused == *run_btn_focus.upcast_ref::<gtk4::Widget>()
                        || focused.is_ancestor(&vbox_vars_focus)
                } else {
                    false
                };

                if is_input_focused {
                    run_cmd_key();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
            _ => glib::Propagation::Proceed,
        }
    });
    window.add_controller(key_ctrl);

    window.set_child(Some(&vbox));
    window.present();

    grab_initial_focus(&entry, &vbox_vars);
}
