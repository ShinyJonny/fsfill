use std::io::{Seek, SeekFrom, Write};
use clap::ArgEnum;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use rand_hc::Hc128Rng;

use crate::{Context, Config};
use crate::usage_map::{UsageMap, AllocStatus};

#[derive(Copy, Clone, Debug, ArgEnum)]
pub enum FillMode {
    Zero,
    #[clap(name = "chacha20")]
    ChaCha20,
    Hc128,
}


/// Zero generator.
/// The generator does nothing. It relies on the assumption that the buffer is already
/// zero-initialised.
struct ZeroGen;

impl ZeroGen {
    fn new() -> Self { Self }
}

impl RngCore for ZeroGen {
    fn next_u32(&mut self) -> u32 { 0 }
    fn next_u64(&mut self) -> u64 { 0 }
    fn fill_bytes(&mut self, _dest: &mut [u8]) {}
    fn try_fill_bytes(&mut self, _dest: &mut [u8]) -> Result<(), rand::Error> { Ok(()) }
}


/// Fills all the free space on the drive.
pub fn fill_free_space(map: &UsageMap, ctx: &mut Context, cfg: &Config) -> anyhow::Result<()>
{
    ctx.logger.log(0, &format!("filling the empty space; fill mode: [{}]", cfg.fill_mode));

    match cfg.fill_mode {
        FillMode::Zero => fill_free_space_with(
            &mut ZeroGen::new(),
            map,
            ctx
        ),
        FillMode::ChaCha20 => fill_free_space_with(
            &mut ChaCha20Rng::from_entropy(),
            map,
            ctx
        ),
        FillMode::Hc128 => fill_free_space_with(
            &mut Hc128Rng::from_entropy(),
            map,
            ctx
        ),
    }
}


/// Fills all the free space on the disk, with a supplied byte generator.
fn fill_free_space_with<T: RngCore>(gen: &mut T, map: &UsageMap, ctx: &mut Context) -> anyhow::Result<()>
{
    // NOTE: IMPORTANT: keep this initialised with zeroes for ZeroGen.
    let mut buf = [0; 4096];
    let mut head = 0;

    for segment in map {
        if segment.status == AllocStatus::Free {
            ctx.drive.seek(SeekFrom::Start(segment.start))?;

            let mut written = 0;

            while written < segment.size() {
                if head == buf.len() {
                    gen.fill_bytes(&mut buf);
                    head = 0;
                }

                let buf_remaining = buf.len() - head;
                let to_write = segment.size() - written;
                let write_size = if to_write < buf_remaining { to_write } else { buf_remaining };

                ctx.drive.write(&buf[head..head + write_size])?;

                written += write_size;
                head += write_size;
            }
        }
    }

    Ok(())
}


// Debug and Display implementations.


impl std::fmt::Display for FillMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        match self {
            Self::Zero =>write!(f, "zero"),
            Self::ChaCha20 => write!(f, "chacha20"),
            Self::Hc128 => write!(f, "HC128"),
        }
    }
}
