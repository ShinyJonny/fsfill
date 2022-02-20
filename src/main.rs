use std::path::PathBuf;
use std::fs::{OpenOptions, File};
use clap::Parser;
use anyhow::anyhow;

mod filesys;
mod array;
mod logger;
mod fill;

use filesys::FsType;
use logger::Logger;
use fill::FillMode;


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

    /// Type of file system
    #[clap(short = 't', long = "type", arg_enum, value_name = "TYPE")]
    fs_type: Option<FsType>,

    /// Set verbosity of the output (can be used multiple times)
    #[clap(short, long, parse(from_occurrences))]
    verbose: u32,

    /// Log file
    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    log_file: Option<PathBuf>,

    /// Mode of disk filling.
    #[clap(short, long, arg_enum, value_name = "MODE")]
    fill_mode: Option<FillMode>
}


fn main()
{
    let args = Args::parse();

    let mut cfg = Config::default();
    cfg.drive_path = args.drive;
    cfg.report_only = args.report_only;
    cfg.verbosity = args.verbose;
    cfg.log_file_path = args.log_file;

    if let Some(mode) = args.fill_mode {
        cfg.fill_mode = mode;
    }

    let mut log_file = None;

    // Create the log file in rw mode.

    if let Some(path) = &cfg.log_file_path {
        let f = OpenOptions::new()
            .create(true)
            .read(false)
            .write(true)
            .open(&path);

        log_file = match f {
            Ok(f) => Some(f),
            Err(e) => {
                eprintln!("error: {}: {}", &path.display(), &e);
                cfg.log_file_path = None;
                None
            }
        };
    }

    // Open the drive in rw mode.

    let drive = OpenOptions::new()
        .create(false)
        .read(true)
        .write(!cfg.report_only)
        .open(&cfg.drive_path);

    let drive = match drive {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {}: {}", &cfg.drive_path.display(), &e);
            return;
        }
    };

    let mut context = Context {
        drive,
        logger: Logger::new(cfg.verbosity, log_file),
    };

    // Set or detect the FS type.

    if let Some(fs_type) = args.fs_type {
        cfg.fs_type = fs_type;
    } else {
        context.logger.log(0, "detecting the file system type");

        cfg.fs_type = match filesys::detect_fs(&mut context) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: {}", &e);
                return;
            }
        };
    }

    context.logger.log(0, "processing the drive");

    // Process the drive.

    if let Err(e) = match cfg.fs_type {
        FsType::Ext2 |
        FsType::Ext3 |
        FsType::Ext4 => filesys::e2fs::process_drive(&mut context, &cfg),
        _ => Err(anyhow!("this filesystem is not implemented yet")),
    } {
        eprintln!("error: {}", &e);
        return;
    };
}


/// Contains configuration options.
#[derive(Clone, Debug)]
pub struct Config {
    pub fs_type: FsType,
    pub drive_path: PathBuf,
    pub log_file_path: Option<PathBuf>,
    pub report_only: bool,
    pub verbosity: u32,
    pub fill_mode: FillMode,
}

impl Default for Config {
    fn default() -> Self
    {
        Self {
            fs_type: FsType::Ext4,
            drive_path: PathBuf::default(),
            log_file_path: None,
            report_only: false,
            verbosity: 0,
            fill_mode: FillMode::Zero,
        }
    }
}


/// Contains shared mutable state.
#[derive(Debug)]
pub struct Context {
    pub drive: File,
    pub logger: Logger,
}
