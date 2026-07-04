use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub enum LogEvent {
    Line(String),
    Finished,
}

pub fn execute(cmd: String, sender: std::sync::mpsc::Sender<LogEvent>) {
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
