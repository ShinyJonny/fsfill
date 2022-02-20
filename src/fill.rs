use clap::ArgEnum;

#[derive(Copy, Clone, Debug, ArgEnum)]
pub enum FillMode {
    Zero,
    ChaCha20,
    Hc128,
}
