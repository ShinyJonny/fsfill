use std::io::Write;
use std::fs::File;

use crate::Config;


/// A simple logger.
#[derive(Debug)]
pub struct Logger {
    verbosity: u32,
    log_file: Option<File>
}

impl Logger {
    /// Create a new logger.
    pub fn new(log_file: Option<File>, cfg: &Config) -> Self
    {
        Self {
            verbosity: cfg.verbosity,
            log_file,
        }
    }

    /// Log a message, with a specified level.
    /// Logs also into the log file, if present.
    pub fn log(&mut self, level: u32, msg: &str)
    {
        if self.verbosity >= level {
            println!("{}", msg);

            if let Some(log_file) = &mut self.log_file {
                write!(log_file, "{}", msg).unwrap_or_else(|_| {
                    eprintln!("couldn't write into the log file");
                });
            }
        }
    }
}
