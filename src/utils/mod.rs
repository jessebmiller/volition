#[derive(Debug, PartialEq, PartialOrd)]
pub enum DebugLevel {
    None,
    Minimal,
    Verbose,
}

pub fn debug_log(level: DebugLevel, message: &str) {
    if level >= DebugLevel::Minimal {
        println!("[DEBUG] {}", message);
    }
}

pub mod git;