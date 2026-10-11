use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use lexopt::ValueExt;

use super::{build, esp, qemu};
use crate::checkpoints::{self, Checkpoints};
use crate::platform::Platform;
use crate::process::{self, Ending};
use crate::workspace::Workspace;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_FILE: &str = "default";

#[derive(Debug)]
struct Request {
    file: String,
    platforms: Vec<Platform>,
    timeout: Duration,
}

pub fn run(arguments: lexopt::Parser) -> anyhow::Result<()> {
    let request = Request::parse(arguments)?;
    let workspace = Workspace::load()?;
    let file = checkpoint_file(&workspace.root, &request.file);
    let text = fs::read_to_string(&file)
        .with_context(|| format!("cannot read the checkpoint file {}", file.display()))?;
    let checkpoints = checkpoints::parse(&text)
        .with_context(|| format!("in the checkpoint file {}", file.display()))?;
    println!(
        "smoke: {} ({} checkpoint(s))",
        file.display(),
        checkpoints.len()
    );
    for platform in request.platforms {
        build::for_platform(&workspace, platform)?;
        esp::populate(&workspace, platform)?;
        probe(&workspace, platform, &checkpoints, request.timeout)?;
    }
    Ok(())
}

/// Checkpoint files live in `smoke/` at the workspace root, one `<name>.txt` per probe
fn checkpoint_file(root: &Path, name: &str) -> PathBuf {
    root.join("smoke").join(format!("{name}.txt"))
}

/// Boots the platform in QEMU and waits for the checkpoints on the serial port
fn probe(
    workspace: &Workspace,
    platform: Platform,
    checkpoints: &[String],
    timeout: Duration,
) -> anyhow::Result<()> {
    let mut command = qemu::command_for(workspace, platform)?;
    let mut progress = Checkpoints::new(checkpoints.to_vec());
    let started = Instant::now();
    let outcome = process::run_until(&mut command, timeout, |line| progress.observe(line))?;
    let elapsed = started.elapsed();
    if progress.all_reached() {
        println!(
            "smoke {}: ok, {} checkpoint(s) in {:.1} s",
            platform.name(),
            progress.count(),
            elapsed.as_secs_f64()
        );
        return Ok(());
    }
    let ending = match outcome.ending {
        Ending::Exited(status) => format!("qemu exited with {status}"),
        Ending::TimedOut => format!("timed out after {} s", timeout.as_secs()),
        Ending::Stopped => "stopped".to_owned(),
    };
    eprint!("{}", outcome.output);
    bail!(
        "smoke {}: {ending}; {}",
        platform.name(),
        progress.report(&outcome.output)
    );
}

impl Request {
    fn parse(mut arguments: lexopt::Parser) -> anyhow::Result<Request> {
        let mut file = None;
        let mut platforms = None;
        let mut timeout = DEFAULT_TIMEOUT;
        while let Some(argument) = arguments.next()? {
            match argument {
                lexopt::Arg::Value(name) => {
                    if file.is_some() {
                        bail!("smoke takes one checkpoint file name, got a second: {name:?}");
                    }
                    file = Some(name.string()?);
                }
                lexopt::Arg::Long("platform") => {
                    let text = arguments.value()?.string()?;
                    platforms = Some(Platform::parse_selection(&text)?);
                }
                lexopt::Arg::Long("timeout") => {
                    let seconds: u64 = arguments
                        .value()?
                        .parse()
                        .context("--timeout takes a number of seconds")?;
                    timeout = Duration::from_secs(seconds);
                }
                other => return Err(other.unexpected().into()),
            }
        }
        Ok(Request {
            file: file.unwrap_or_else(|| DEFAULT_FILE.to_owned()),
            platforms: platforms.unwrap_or_else(|| Platform::ALL.to_vec()),
            timeout,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use super::{DEFAULT_FILE, DEFAULT_TIMEOUT, Request, checkpoint_file};
    use crate::platform::Platform;

    #[test]
    fn parse_takes_a_file_name_platform_and_timeout() {
        let request = Request::parse(lexopt::Parser::from_args([
            "--platform",
            "aarch64",
            "loader",
            "--timeout",
            "5",
        ]))
        .unwrap();
        assert_eq!(request.file, "loader");
        assert_eq!(request.platforms, [Platform::Aarch64]);
        assert_eq!(request.timeout, Duration::from_secs(5));
    }

    #[test]
    fn parse_defaults_to_the_default_file_all_platforms_and_two_minutes() {
        let request = Request::parse(lexopt::Parser::from_args(Vec::<String>::new())).unwrap();
        assert_eq!(request.file, DEFAULT_FILE);
        assert_eq!(request.platforms, Platform::ALL);
        assert_eq!(request.timeout, DEFAULT_TIMEOUT);
    }

    #[test]
    fn parse_rejects_a_second_file_name() {
        let error = Request::parse(lexopt::Parser::from_args(["one", "two"])).unwrap_err();
        assert!(error.to_string().contains("one checkpoint file name"));
    }

    #[test]
    fn parse_rejects_a_timeout_that_is_not_a_number() {
        let error = Request::parse(lexopt::Parser::from_args(["--timeout", "soon"])).unwrap_err();
        assert!(error.to_string().contains("--timeout"));
    }

    #[test]
    fn checkpoint_files_live_under_smoke_at_the_root() {
        assert_eq!(
            checkpoint_file(Path::new("/workspace"), "loader"),
            PathBuf::from("/workspace/smoke/loader.txt")
        );
    }
}
