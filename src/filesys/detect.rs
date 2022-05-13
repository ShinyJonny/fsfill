use std::io::{Seek, SeekFrom,};
use bincode::{Options, DefaultOptions};
use crate::Context;
use super::FsType;
use super::e2fs;


/// Attempts to detect the file system.
pub fn detect_fs(context: &mut Context) -> anyhow::Result<Option<FsType>>
{
    if detect_e2fs(context)? {
        return Ok(Some(FsType::Ext2));
    }

    Ok(None)
}


/// Attempts to detect the ext2/3/4 file system.
fn detect_e2fs(context: &mut Context) -> anyhow::Result<bool>
{
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    context.drive.seek(SeekFrom::Start(1024))?;
    let sb: e2fs::SuperBlock = bincode_opt.deserialize_from(&context.drive)?;

    // Magic value.
    if sb.s_magic != 0xef53 {
        return Ok(false);
    }

    // Check for invalid fields.

    if sb.s_state == 0 || sb.s_state >> 3 != 0 {
        return Ok(false);
    }

    if sb.s_errors == 0 || sb.s_errors > 3 {
        return Ok(false);
    }

    if sb.s_rev_level > 1 {
        return Ok(false);
    }

    Ok(true)
}
