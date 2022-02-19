use std::io::{Seek, SeekFrom,};
use anyhow::anyhow;
use bincode::{Options, DefaultOptions};
use crate::Context;
use super::FsType;
use super::e2fs;


// TODO
/// Attempts to detect the file system.
pub fn detect_fs(context: &mut Context) -> anyhow::Result<FsType>
{
    if let Some(v) = detect_e2fs(context)? { return Ok(v); }

    Err(anyhow!("Unknown file system"))
}


/// Attempts to detect the ext2/3/4 file system.
fn detect_e2fs(context: &mut Context) -> anyhow::Result<Option<FsType>>
{
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    context.drive.seek(SeekFrom::Start(1024))?;
    let sb: e2fs::SuperBlock = bincode_opt.deserialize_from(&context.drive)?;

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
