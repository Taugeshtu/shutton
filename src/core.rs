use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub enum LogEvent {
    Line(String),
    Finished,
}

pub fn parse_vars(text: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let mut name = String::new();
            while let Some(&next_c) = chars.peek() {
                if next_c.is_alphanumeric() || next_c == '_' {
                    name.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if !name.is_empty() && !vars.contains(&name) {
                vars.push(name);
            }
        }
    }
    vars
}

pub fn execute(config: crate::storage::Config, sender: std::sync::mpsc::Sender<LogEvent>) {
    // 1. Save config to binary
    if let Err(e) = crate::storage::patch_config(&config) {
        eprintln!("Error saving config to binary: {}", e);
    }

    // 2. Perform variable substitution for execution
    let mut cmd = config.main_cmd.clone();
    let parsed_vars = parse_vars(&config.main_cmd);
    for (i, var) in parsed_vars.iter().enumerate() {
        if let Some(val) = config.var_values.get(i) {
            cmd = cmd.replace(&format!("%{}", var), val);
        }
    }

    // 3. Spawn background thread
    std::thread::spawn(move || {
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
}
