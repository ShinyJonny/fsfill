use std::io::{Seek, SeekFrom,};
use anyhow::anyhow;
use bincode::{Options, DefaultOptions};
use crate::Context;
use super::FsType;
use super::ext2;

// TODO
/// Attempts to detect the file system.
pub fn detect_fs(context: &mut Context) -> anyhow::Result<FsType>
{
    if let Some(v) = detect_ext2(context)? { return Ok(v); }

    Err(anyhow!("Unknown file system"))
}

fn detect_ext2(context: &mut Context) -> anyhow::Result<Option<FsType>>
{
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    context.drive.seek(SeekFrom::Start(1024))?;
    let sb: ext2::SuperBlock = bincode_opt.deserialize_from(&context.drive)?;

    if sb.s_magic != 0xef53 {
        return Ok(None);
    }

    if sb.s_state == 0 || sb.s_state >> 3 != 0 {
        return Ok(None);
    }

    if sb.s_errors == 0 || sb.s_errors > 3 {
        return Ok(None);
    }

    if sb.s_rev_level > 1 {
        return Ok(None);
    }

    Ok(Some(FsType::Ext2))
}
