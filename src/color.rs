/// Phosphor-green terminal styling.
/// When color is enabled, wraps text in ANSI escape codes for green (#00ff41-ish).

const GREEN: &str = "\x1b[38;2;0;255;200m";
const BRIGHT_GREEN: &str = "\x1b[38;2;160;255;230m";
const DIM_GREEN: &str = "\x1b[38;2;0;128;100m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

pub struct Theme {
    pub color: bool,
}

impl Theme {
    pub fn new(color: bool) -> Self {
        Self { color }
    }

    pub fn green(&self, text: &str) -> String {
        if self.color {
            format!("{GREEN}{text}{RESET}")
        } else {
            text.to_string()
        }
    }

    pub fn bright(&self, text: &str) -> String {
        if self.color {
            format!("{BRIGHT_GREEN}{BOLD}{text}{RESET}")
        } else {
            text.to_string()
        }
    }

    pub fn dim(&self, text: &str) -> String {
        if self.color {
            format!("{DIM_GREEN}{text}{RESET}")
        } else {
            text.to_string()
        }
    }

    pub fn header(&self, text: &str) -> String {
        if self.color {
            format!("{GREEN}{BOLD}{text}{RESET}")
        } else {
            text.to_string()
        }
    }

    pub fn value(&self, text: &str) -> String {
        if self.color {
            format!("{BRIGHT_GREEN}{text}{RESET}")
        } else {
            text.to_string()
        }
    }
}
