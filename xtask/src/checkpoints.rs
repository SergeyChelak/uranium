use std::collections::VecDeque;
use std::fmt::Write;

use anyhow::bail;

/// Reads a checkpoint file: one line per checkpoint in the expected order; blank lines and
/// lines starting with `#` are ignored
pub fn parse(text: &str) -> anyhow::Result<Vec<String>> {
    let checkpoints: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    if checkpoints.is_empty() {
        bail!("the checkpoint file has no checkpoints, only blank lines and comments");
    }
    Ok(checkpoints)
}

/// Lines expected on the serial port in this order; each is matched as a substring.
/// Only the next pending checkpoint can match, so a line repeated in a loop never
/// advances past it, and nothing can be reached twice
pub struct Checkpoints {
    reached: Vec<String>,
    pending: VecDeque<String>,
}

impl Checkpoints {
    pub fn new(expected: Vec<String>) -> Checkpoints {
        Checkpoints {
            reached: Vec::new(),
            pending: expected.into(),
        }
    }

    /// Feeds one line of output; true once the last checkpoint has been reached
    pub fn observe(&mut self, line: &str) -> bool {
        if self
            .pending
            .front()
            .is_some_and(|next| line.contains(next.as_str()))
            && let Some(next) = self.pending.pop_front()
        {
            self.reached.push(next);
        }
        self.all_reached()
    }

    pub fn all_reached(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn count(&self) -> usize {
        self.reached.len() + self.pending.len()
    }

    /// Explains a run that ended before the last checkpoint, given everything it printed
    pub fn report(&self, output: &str) -> String {
        let mut report = format!(
            "reached {} of {} checkpoints",
            self.reached.len(),
            self.count()
        );
        for line in &self.reached {
            let _ = write!(report, "\n  reached: {line}");
        }
        if let Some(missing) = self.pending.front() {
            let _ = write!(report, "\n  missing: {missing}");
            if output.contains(missing.as_str()) {
                match self.reached.last() {
                    Some(previous) => {
                        let _ = write!(
                            report,
                            " (it appears in the output, but not after `{previous}`)"
                        );
                    }
                    None => report.push_str(" (it appears in the output)"),
                }
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::{Checkpoints, parse};

    #[test]
    fn parse_keeps_trimmed_lines_in_order_and_skips_blanks_and_comments() {
        let text = "# the loader\n\n  uranium loader started  \n\n# the kernel\nkernel entered\n";
        assert_eq!(
            parse(text).unwrap(),
            ["uranium loader started", "kernel entered"]
        );
    }

    #[test]
    fn parse_rejects_a_file_without_checkpoints() {
        let error = parse("# nothing here\n\n").unwrap_err();
        assert!(error.to_string().contains("no checkpoints"));
    }

    fn checkpoints(lines: &[&str]) -> Checkpoints {
        Checkpoints::new(lines.iter().map(|line| (*line).to_owned()).collect())
    }

    #[test]
    fn reaches_checkpoints_in_order_and_stops_at_the_last() {
        let mut checkpoints = checkpoints(&["first", "second"]);
        assert!(!checkpoints.observe("noise"));
        assert!(!checkpoints.observe("the first line"));
        assert!(!checkpoints.observe("more noise"));
        assert!(checkpoints.observe("the second line"));
        assert!(checkpoints.all_reached());
    }

    #[test]
    fn a_later_checkpoint_does_not_count_before_an_earlier_one() {
        let mut checkpoints = checkpoints(&["first", "second"]);
        assert!(!checkpoints.observe("second"));
        assert!(!checkpoints.observe("first"));
        assert!(!checkpoints.all_reached());
        let report = checkpoints.report("second\nfirst\n");
        assert!(report.contains("reached 1 of 2 checkpoints"));
        assert!(report.contains("reached: first"));
        assert!(
            report.contains("missing: second (it appears in the output, but not after `first`)")
        );
    }

    #[test]
    fn a_checkpoint_repeated_in_a_loop_never_advances_past_itself() {
        let mut checkpoints = checkpoints(&["first", "second"]);
        for _ in 0..100 {
            assert!(!checkpoints.observe("first"));
        }
        assert!(checkpoints.observe("second"));
    }

    #[test]
    fn nothing_changes_after_the_last_checkpoint() {
        let mut checkpoints = checkpoints(&["only"]);
        assert!(checkpoints.observe("only"));
        assert!(checkpoints.observe("only"));
        assert!(checkpoints.observe("anything"));
        assert_eq!(checkpoints.count(), 1);
    }

    #[test]
    fn report_names_the_first_missing_checkpoint() {
        let checkpoints = checkpoints(&["first", "second"]);
        assert_eq!(
            checkpoints.report("nothing\n"),
            "reached 0 of 2 checkpoints\n  missing: first"
        );
    }

    #[test]
    fn report_says_when_the_first_checkpoint_appears_at_all() {
        let checkpoints = checkpoints(&["first"]);
        assert!(
            checkpoints
                .report("first\n")
                .ends_with("missing: first (it appears in the output)")
        );
    }

    #[test]
    fn a_single_checkpoint_is_the_whole_run() {
        let mut checkpoints = checkpoints(&["uranium loader started"]);
        assert!(checkpoints.observe("uranium loader started"));
        assert_eq!(checkpoints.count(), 1);
    }
}
