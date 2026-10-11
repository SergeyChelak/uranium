use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context;

use crate::cargo::describe;

/// How a run under `run_until` came to an end
#[derive(Debug)]
pub enum Ending {
    /// The `stop` callback asked for the end; the child was killed
    Stopped,
    Exited(ExitStatus),
    /// The timeout passed first; the child was killed
    TimedOut,
}

pub struct Outcome {
    /// Every line of standard output seen, without carriage returns
    pub output: String,
    pub ending: Ending,
}

/// Runs the command with its standard output captured line by line. `stop` sees every
/// line as it arrives and ends the run early by returning true. Standard error stays on
/// this terminal
pub fn run_until(
    command: &mut Command,
    timeout: Duration,
    mut stop: impl FnMut(&str) -> bool,
) -> anyhow::Result<Outcome> {
    let description = describe(command);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start `{description}`"))?;
    let stdout = child
        .stdout
        .take()
        .expect("standard output was requested as a pipe");
    let (sender, receiver) = mpsc::channel();
    // Not joined: a killed child may leave grandchildren that keep the pipe open (a shell's
    // `sleep`), and the reader ends on its own once they are gone
    thread::spawn(move || forward_lines(stdout, &sender));
    let deadline = Instant::now() + timeout;
    let mut output = String::new();
    let ending = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match receiver.recv_timeout(remaining) {
            Ok(line) => {
                let done = stop(&line);
                output.push_str(&line);
                output.push('\n');
                if done {
                    kill(&mut child)?;
                    break Ending::Stopped;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                let status = child
                    .wait()
                    .with_context(|| format!("failed to wait for `{description}`"))?;
                break Ending::Exited(status);
            }
            Err(RecvTimeoutError::Timeout) => {
                kill(&mut child)?;
                break Ending::TimedOut;
            }
        }
    };
    Ok(Outcome { output, ending })
}

fn forward_lines(stdout: impl Read, sender: &Sender<String>) {
    let mut reader = BufReader::new(stdout);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let line = String::from_utf8_lossy(&bytes);
                let line = line.trim_end_matches(['\r', '\n']).to_owned();
                if sender.send(line).is_err() {
                    return;
                }
            }
        }
    }
}

fn kill(child: &mut Child) -> anyhow::Result<()> {
    child.kill().context("failed to kill the child process")?;
    child.wait().context("failed to reap the killed child")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::{Duration, Instant};

    use super::{Ending, run_until};

    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }

    #[test]
    fn stops_at_the_requested_line_and_kills_the_child() {
        let started = Instant::now();
        let outcome = run_until(
            &mut shell("echo one; echo two; sleep 30"),
            Duration::from_secs(20),
            |line| line == "two",
        )
        .unwrap();
        assert!(matches!(outcome.ending, Ending::Stopped));
        assert_eq!(outcome.output, "one\ntwo\n");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn times_out_and_kills_the_child() {
        let started = Instant::now();
        let outcome = run_until(&mut shell("sleep 30"), Duration::from_millis(200), |_| {
            false
        })
        .unwrap();
        assert!(matches!(outcome.ending, Ending::TimedOut));
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn collects_output_of_a_finished_child() {
        let outcome = run_until(
            &mut shell(r#"printf 'hello\r\nworld'"#),
            Duration::from_secs(20),
            |_| false,
        )
        .unwrap();
        assert!(matches!(outcome.ending, Ending::Exited(status) if status.success()));
        assert_eq!(outcome.output, "hello\nworld\n");
    }

    #[test]
    fn reports_a_failing_exit_status() {
        let outcome = run_until(&mut shell("exit 3"), Duration::from_secs(20), |_| false).unwrap();
        assert!(matches!(outcome.ending, Ending::Exited(status) if status.code() == Some(3)));
    }
}
