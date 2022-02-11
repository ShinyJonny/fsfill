use std::path::PathBuf;
use std::fs::{OpenOptions, File};
use clap::Parser;

mod filesys;

use filesys::FsType;

#[derive(Debug, Parser)]
struct Args {
    /// Display help
    #[clap(short, long)]
    help: bool,

    /// Display the version of the program
    #[clap(short = 'V', long)]
    version: bool,

    /// Drive path
    #[clap(short, long, parse(from_os_str), value_name = "DRIVE")]
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

    ///// List supported file system types
    //#[clap(long)]
    //list_types: bool,
}

fn main()
{
    let args = Args::parse();

    let mut cfg = Config::default();
    cfg.drive_path = args.drive;
    cfg.report_only = args.report_only;
    cfg.verbosity = args.verbose;
    cfg.log_file_path = args.log_file;

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
                eprintln!("error: {}: {}", &path.display(), e.to_string());
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
            eprintln!("error: {}: {}", &cfg.drive_path.display(), e.to_string());
            return;
        }
    };

    let mut context = Context {
        log_file,
        drive,
    };

    // Set or detect the FS type.

    if let Some(fs_type) = args.fs_type {
        cfg.fs_type = fs_type;
    } else {
        cfg.fs_type = match filesys::detect_fs(&mut context, &cfg) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: {}", e.to_string());
                return;
            }
        };
    }

    let status = match cfg.fs_type {
        FsType::Ext4 => filesys::process_ext4(&mut context, &cfg),
    };

    if let Err(e) = status {
        eprintln!("error: {}", e.to_string());
        return;
    }
}

/// Contains configuration options.
#[derive(Debug)]
pub struct Config {
    pub fs_type: FsType,
    pub drive_path: PathBuf,
    pub log_file_path: Option<PathBuf>,
    pub report_only: bool,
    pub verbosity: u32,
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
        }
    }
}

/// Contains shared mutable state.
#[derive(Debug)]
pub struct Context {
    pub log_file: Option<File>,
    pub drive: File,
}
