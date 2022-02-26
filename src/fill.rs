use clap::ArgEnum;

#[derive(Copy, Clone, Debug, ArgEnum)]
pub enum FillMode {
    Zero,
    #[clap(name = "chacha20")]
    ChaCha20,
    Hc128,
}
