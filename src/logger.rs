use std::io::Write;
use std::fs::File;

use crate::Config;


/// A simple logger.
#[derive(Debug)]
pub struct Logger {
    verbosity: u32,
    log_file: Option<File>,
    cmd_name: String,
}

impl Logger {
    /// Create a new logger.
    pub fn new(log_file: Option<File>, cfg: &Config) -> Self
    {
        Self {
            verbosity: cfg.verbosity,
            log_file,
            cmd_name: cfg.cmd_name.clone(),
        }
    }

    /// Log a message, with a specified level.
    /// Logs also into the log file, if present.
    pub fn log(&mut self, level: u32, msg: &str)
    {
        if self.verbosity >= level {
            eprint!("{}", msg);

            if let Some(log_file) = &mut self.log_file {
                write!(log_file, "{}", msg).unwrap_or_else(|_| {
                    eprintln!("{}: couldn't write into the log file", self.cmd_name);
                });
            }
        }
    }

    /// Log a message line, with a specified level.
    /// Logs also into the log file, if present.
    pub fn logln(&mut self, level: u32, msg: &str)
    {
        if self.verbosity >= level {
            eprintln!("{}", msg);

            if let Some(log_file) = &mut self.log_file {
                writeln!(log_file, "{}", msg).unwrap_or_else(|_| {
                    eprintln!("{}: couldn't write into the log file", self.cmd_name);
                });
            }
        }
    }
}
