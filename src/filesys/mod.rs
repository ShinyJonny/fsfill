use clap::ArgEnum;

mod detect;

pub mod e2fs;
pub use detect::detect_fs;

/// Supported file system types.
#[derive(Copy, Clone, Debug, ArgEnum)]
pub enum FsType {
    Ext2,
    Ext3,
    Ext4,
}
