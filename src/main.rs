use std::path::PathBuf;
use std::fs::{OpenOptions, File};
use clap::Parser;
use anyhow::anyhow;

mod filesys;
mod array;
mod logger;
mod fill;
mod usage_map;
mod util;
mod bitmap;

use filesys::FsType;
use logger::Logger;
use fill::FillMode;

/// Command line argument configuration.
#[derive(Debug, Parser)]
#[clap(version)]
struct Args {
    /// Display help
    #[clap(short, long)]
    help: bool,

    /// Display the version of the program
    #[clap(short = 'V', long)]
    version: bool,

    /// Drive path
    #[clap(parse(from_os_str), value_name = "DRIVE")]
    drive: PathBuf,

    /// Report only, do not modify the file system
    #[clap(short, long)]
    report_only: bool,

    /// Prettify the output (when using --report-only)
    #[clap(short, long)]
    pretty: bool,

    /// Type of file system
    #[clap(short = 't', long = "type", arg_enum, value_name = "TYPE")]
    fs_type: Option<FsType>,

    /// Ignore the recovery error
    #[clap(short = 'R', long)]
    ignore_recovery: bool,

    /// Ignore read-only flags
    #[clap(short = 'O', long)]
    ignore_readonly: bool,

    /// Set verbosity of the output (can be used multiple times)
    #[clap(short, long, parse(from_occurrences))]
    verbose: u32,

    /// Log file
    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    log_file: Option<PathBuf>,

    /// Mode of disk filling
    #[clap(short, long, arg_enum, value_name = "MODE")]
    fill_mode: Option<FillMode>
}

fn main()
{
    let args = Args::parse();

    // Process the command line arguments.

    let mut cfg = Config::default();
    cfg.cmd_name = std::env::args().nth(0).unwrap();
    cfg.drive_path = args.drive;
    cfg.report_only = args.report_only;
    cfg.verbosity = args.verbose;
    cfg.log_file_path = args.log_file;
    cfg.ignore_recovery = args.ignore_recovery;
    cfg.ignore_readonly = args.ignore_readonly;
    cfg.pretty = args.pretty;

    if let Some(mode) = args.fill_mode {
        cfg.fill_mode = mode;
    }

    let mut log_file = None;

    // Create or open the log file in append mode.

    if let Some(path) = &cfg.log_file_path {
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .read(false)
            .open(&path);

        log_file = match f {
            Ok(f) => Some(f),
            Err(e) => {
                eprintln!("{}: {}: {}", cfg.cmd_name, &path.display(), &e);
                cfg.log_file_path = None;
                None
            }
        };
    }

    let mut logger = Logger::new(log_file, &cfg);

    // Open the drive.

    let drive = OpenOptions::new()
        .create(false)
        .read(true)
        .write(!cfg.report_only)
        .open(&cfg.drive_path);

    let drive = match drive {
        Ok(f) => f,
        Err(e) => {
            logger.logln(0, &format!("{}: {}: {}", cfg.cmd_name, &cfg.drive_path.display(), &e));
            std::process::exit(1);
        }
    };

    let mut context = Context {
        drive,
        logger,
    };

    // Set or detect the FS type.

    cfg.fs_type = if let Some(fs_type) = args.fs_type {
        fs_type
    } else {
        context.logger.log(0, "=== detecting the file system type: ");

        let fs_type = match filesys::detect_fs(&mut context) {
            Ok(fs_option) => {
                if let Some(fs_type) = fs_option {
                    fs_type
                } else {
                    context.logger.logln(0, "unknown");
                    context.logger.logln(0, &format!("{}: aborting", cfg.cmd_name));
                    std::process::exit(1);
                }
            },
            Err(e) => {
                context.logger.logln(0, &format!("{}: {}", cfg.cmd_name, &e));
                std::process::exit(1);
            }
        };

        match fs_type {
            FsType::Ext2 => context.logger.logln(0, "ext2"),
            FsType::Ext3 => context.logger.logln(0, "ext3"),
            FsType::Ext4 => context.logger.logln(0, "ext4"),
        }

        fs_type
    };

    // Scan the drive.

    context.logger.logln(0, "=== scanning the drive");

    let map = match cfg.fs_type {
        FsType::Ext2 |
        FsType::Ext3 |
        FsType::Ext4 => filesys::e2fs::scan_drive(&mut context, &cfg),
        #[allow(unreachable_patterns)]
        _ => Err(anyhow!("this filesystem is not implemented yet")),
    }.unwrap_or_else(|e| {
        context.logger.logln(0, &format!("{}: {}", cfg.cmd_name, &e));
        std::process::exit(1);
    });

    // Report or fill.

    if cfg.report_only {
        // Print out the usage map in JSON format.

        if cfg.pretty {
            println!("{}", serde_json::to_string_pretty(&map).unwrap());
        } else {
            println!("{}", serde_json::to_string(&map).unwrap());
        }
    } else {
        // Fill the free space.

        context.logger.log(0, "=== filling the free space");
        context.logger.logln(0, &format!("; fill mode: {}", cfg.fill_mode));

        if let Err(e) = fill::fill_free_space(&map, &mut context, &cfg) {
            context.logger.logln(0, &format!("{}: {}", cfg.cmd_name, &e));
            std::process::exit(1);
        }
    }
}

/// Configuration options.
#[derive(Clone, Debug)]
pub struct Config {
    pub cmd_name: String,
    pub fs_type: FsType,
    pub drive_path: PathBuf,
    pub log_file_path: Option<PathBuf>,
    pub report_only: bool,
    pub verbosity: u32,
    pub fill_mode: FillMode,
    pub ignore_recovery: bool,
    pub ignore_readonly: bool,
    pub pretty: bool,
}

impl Default for Config {
    fn default() -> Self
    {
        Self {
            cmd_name: String::from("fsfill"),
            fs_type: FsType::Ext4,
            drive_path: PathBuf::default(),
            log_file_path: None,
            report_only: true,
            verbosity: 0,
            fill_mode: FillMode::Zero,
            ignore_recovery: false,
            ignore_readonly: false,
            pretty: false,
        }
    }
}

/// Shared mutable state.
#[derive(Debug)]
pub struct Context {
    pub drive: File,
    pub logger: Logger,
}
