/// Computes the block size from `s_log_block_size`.
/// 2 ^ (10 + s_log_block_size)
#[macro_export]
macro_rules! bs {
    ($bs:expr) => {
        u64::pow(2, 10 + $bs)
    };
}

/// The size of a group descriptor for buffer allocation.
/// The larger one is picked to avoid de/serialisation problems.
#[macro_export]
macro_rules! alloc_desc_size {
    ($size:expr) => {
        if $size as usize > GROUP_DESC_STRUCT_SIZE {
            $size as usize
        } else {
            GROUP_DESC_STRUCT_SIZE
        }
    };
}

/// The size of an inode for buffer allocation.
/// The larger one is picked to avoid de/serialisation problems.
#[macro_export]
macro_rules! alloc_inode_size {
    ($size:expr) => {
        if $size as usize > INODE_STRUCT_SIZE {
            $size as usize
        } else {
            INODE_STRUCT_SIZE
        }
    };
}
