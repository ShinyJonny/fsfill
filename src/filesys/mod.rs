use std::io::{Seek, SeekFrom, Read};
use clap::ArgEnum;
use anyhow::anyhow;
use crate::{Context, Config};

pub mod e2fs;

/// Supported file system types.
#[derive(Clone, Debug, ArgEnum)]
pub enum FsType {
    Ext2,
    Ext3,
    Ext4,
    Btrfs,
    Fat,
    Exfat,
    NTFS,
}

/// Attempts to detect the file system.
pub fn detect_fs(context: &mut Context, _cfg: &Config) -> anyhow::Result<FsType>
{
    let mut buffer: [u8; 32] = [0; 32];

    // Ext4 detection.

    context.drive.seek(SeekFrom::Start(1024 + 0x38))?;
    context.drive.read_exact(&mut buffer[..2])?;

    if buffer[..2] == [0x53, 0xef] {
        return Ok(FsType::Ext4);
    }

    Err(anyhow!("Unknown file system"))
}
