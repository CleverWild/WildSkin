// In-game logger. The GUI Logger tab displays `lines()`, but nothing emits
// log lines yet, so `add_log`/`clear`/`auto_scroll` read as dead until the
// call sites are wired up.
pub struct Logger {
    lines: Vec<String>,
    #[expect(
        dead_code,
        reason = "GUI auto-scroll toggle for the Logger tab, not wired yet, see comment above"
    )]
    pub auto_scroll: bool,
}

impl Logger {
    pub const fn new() -> Self {
        Self {
            lines: Vec::new(),
            auto_scroll: true,
        }
    }

    #[allow(
        dead_code,
        reason = "logging entry point, not wired at call sites yet but exercised by unit tests, see comment above"
    )]
    pub fn add_log(&mut self, line: impl AsRef<str>) {
        self.lines.push(line.as_ref().to_owned());
    }

    #[allow(
        dead_code,
        reason = "logging entry point, not wired at call sites yet but exercised by unit tests, see comment above"
    )]
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_log_appends_a_line() {
        let mut log = Logger::new();
        log.add_log("GameClient found!");
        assert_eq!(log.lines().collect::<Vec<_>>(), vec!["GameClient found!"]);
    }

    #[test]
    fn multiple_calls_accumulate_in_order() {
        let mut log = Logger::new();
        log.add_log("first");
        log.add_log("second");
        assert_eq!(log.lines().collect::<Vec<_>>(), vec!["first", "second"]);
    }

    #[test]
    fn clear_empties_the_buffer() {
        let mut log = Logger::new();
        log.add_log("will be cleared");
        log.clear();
        assert_eq!(log.lines().count(), 0);
    }
}
