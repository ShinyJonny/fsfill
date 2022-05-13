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
    match cfg.fill_mode {
        FillMode::Zero => fill_free_space_with(
            &mut ZeroGen::new(),
            map,
            &mut ctx.drive
        ),
        FillMode::ChaCha20 => fill_free_space_with(
            &mut ChaCha20Rng::from_entropy(),
            map,
            &mut ctx.drive
        ),
        FillMode::Hc128 => fill_free_space_with(
            &mut Hc128Rng::from_entropy(),
            map,
            &mut ctx.drive
        ),
    }
}


/// Fills all the free space on the disk, using a supplied byte generator.
fn fill_free_space_with<R, W>(gen: &mut R, map: &UsageMap, drive: &mut W) -> anyhow::Result<()>
where
    R: RngCore,
    W: Write + Seek
{
    // NOTE: IMPORTANT: keep this initialised with zeroes for ZeroGen.
    let mut buf = [0; 4096];
    // Buffer head.
    let mut head = 0;
    gen.fill_bytes(&mut buf);

    // Iterate through the segments in the map.
    // If a segment is free, fill the corresponding drive addresses with the bytes from the buffer.
    // The buffer is refilled with the byte generator when it is used up.

    for segment in map {
        if segment.status == AllocStatus::Free {
            drive.seek(SeekFrom::Start(segment.start))?;

            let mut written = 0;

            while written < segment.size() {
                if head == buf.len() {
                    gen.fill_bytes(&mut buf);
                    head = 0;
                }

                let buf_remaining = buf.len() - head;
                let to_write = segment.size() - written;
                let write_size = if to_write < buf_remaining { to_write } else { buf_remaining };

                drive.write(&buf[head..head + write_size])?;

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


// Tests


#[cfg(test)]
mod tests {
    #[test]
    fn fill()
    {
        use super::*;

        let mut f = std::io::Cursor::new(vec![0xffu8; 4096 * 10]);
        let len = f.seek(SeekFrom::End(0)).unwrap();

        let mut map = UsageMap::new(len);

        map.update(2, 79, AllocStatus::Used);
        map.update(201, 335, AllocStatus::Used);
        map.update(700, 1000, AllocStatus::Used);
        map.update(5000, 7028, AllocStatus::Used);
        map.update(20000, 2, AllocStatus::Used);
        map.update(20229, 33, AllocStatus::Used);

        super::fill_free_space_with(&mut ZeroGen::new(), &map, &mut f).unwrap();

        for seg in map.0.iter().filter(|s| { s.status == AllocStatus::Free }) {
            for b in &f.get_ref()[seg.start as usize..seg.end as usize] {
                assert_eq!(*b, 0u8);
            }
        }

        for seg in map.0.iter().filter(|s| { s.status == AllocStatus::Used }) {
            for b in &f.get_ref()[seg.start as usize..seg.end as usize] {
                assert_eq!(*b, 0xffu8);
            }
        }
    }
}
