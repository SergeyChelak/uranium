use crate::Console;

/// A console failure is not worth stopping the boot: the smoke test will
/// time out instead
#[cfg(feature = "checkpoints")]
pub fn checkpoint(console: &mut impl Console, name: &str) {
    let _ = console.write_line(name);
}

#[cfg(not(feature = "checkpoints"))]
pub fn checkpoint(_console: &mut impl Console, _name: &str) {}

#[cfg(test)]
mod tests {
    use super::checkpoint;
    use crate::{Console, ConsoleError};
    use std::string::String;

    struct Recorder(String);

    impl Console for Recorder {
        fn write_str(&mut self, text: &str) -> Result<(), ConsoleError> {
            self.0.push_str(text);
            Ok(())
        }
    }

    #[cfg(feature = "checkpoints")]
    #[test]
    fn checkpoint_writes_the_name() {
        let mut console = Recorder(String::new());
        checkpoint(&mut console, "loader: started");
        assert_eq!(console.0, "loader: started\n");
    }

    #[cfg(not(feature = "checkpoints"))]
    #[test]
    fn checkpoint_is_silent_without_the_feature() {
        let mut console = Recorder(String::new());
        checkpoint(&mut console, "loader: started");
        assert_eq!(console.0, "");
    }
}
