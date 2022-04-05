use clap::ArgEnum;
use anyhow::bail;
use crate::{Context, Config};
use crate::usage_map::UsageMap;

#[derive(Copy, Clone, Debug, ArgEnum)]
pub enum FillMode {
    Zero,
    #[clap(name = "chacha20")]
    ChaCha20,
    Hc128,
}


pub fn fill_drive(_map: &UsageMap, _context: &mut Context, _cfg: &Config) -> anyhow::Result<()>
{
    bail!("dummy")
}
