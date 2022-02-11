use std::io::{Seek, SeekFrom, Read, ErrorKind};
use crate::{Context, Config};

extern crate clap;

use clap::ArgEnum;

/// Supported file system types.
#[derive(Clone, Debug, ArgEnum)]
pub enum FsType {
    Ext4,
}

/// Attempts to detect the file system.
pub fn detect_fs(context: &mut Context, _cfg: &Config) -> Result<FsType, std::io::Error>
{
    let mut buffer: [u8; 32] = [0; 32];

    context.drive.seek(SeekFrom::Start(1024 + 0x38))?;
    context.drive.read_exact(&mut buffer[..2])?;

    if buffer[..2] == [0x53, 0xef] {
        return Ok(FsType::Ext4);
    }

    Err(std::io::Error::new(ErrorKind::InvalidData, "Unknown file system"))
}

// TODO
/// The main procedure for processing Ext4 file systems.
pub fn process_ext4(_context: &mut Context, _cfg: &Config) -> Result<(), std::io::Error>
{
    Ok(())
}
