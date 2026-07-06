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
    // 2. Perform variable substitution for execution
    let mut cmd = config.main_cmd.clone();
    let parsed_vars = parse_vars(&config.main_cmd);
    for (i, var) in parsed_vars.iter().enumerate() {
        if let Some(val) = config.var_values.get(i) {
            cmd = cmd.replace(&format!("%{}", var), val);
        }
    }

    // Spawn background thread
    std::thread::spawn(move || {
        let pid = std::process::id();
        let temp_dir = std::env::temp_dir().join(format!("shutton-sudo-{}", pid));
        
        let mut setup_ok = false;
        if std::fs::create_dir_all(&temp_dir).is_ok() {
            let sudo_script = temp_dir.join("sudo");
            if std::fs::write(&sudo_script, "#!/bin/sh\nexec pkexec \"$@\"\n").is_ok() {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&sudo_script) {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o755);
                    if std::fs::set_permissions(&sudo_script, perms).is_ok() {
                        setup_ok = true;
                    }
                }
            }
        }

        let mut child_cmd = Command::new("sh");
        child_cmd.arg("-c")
            .arg(format!("{} 2>&1", cmd))
            .stdout(Stdio::piped());

        if setup_ok {
            let current_path = std::env::var_os("PATH").unwrap_or_default();
            let mut new_path = temp_dir.clone().into_os_string();
            new_path.push(":");
            new_path.push(current_path);
            child_cmd.env("PATH", new_path);
        }

        use std::os::unix::process::CommandExt;
        unsafe {
            child_cmd.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }

        let child = child_cmd.spawn();

        match child {
            Ok(mut child) => {
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if let Ok(mut l) = line {
                            l.push('\n');
                            let clean = strip_ansi(&l);
                            let _ = sender.send(LogEvent::Line(clean));
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

        if setup_ok {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }
    });
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next_c) = chars.peek() {
                    chars.next();
                    if next_c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}
