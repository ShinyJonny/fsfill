use serde::{Deserialize, Serialize};

use crate::usage_map::UsageMap;
use crate::Context;

use super::inode::{Inode, Osd2};
use super::Fs;


// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4_extents.h
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Extent {
    pub ee_block: u32,    // first logical block extent covers
    pub ee_len: u16,      // number of blocks covered by extent
    pub ee_start_hi: u16, // high 16 bits of physical block
    pub ee_start_lo: u32, // low 32 bits of physical block
}


// https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4_extents.h
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ExtentHeader {
    pub eh_magic: u16,      // probably will support different formats
    pub eh_entries: u16,    // number of valid entries
    pub eh_max: u16,        // capacity of store in entries
    pub eh_depth: u16,      // has tree real underlying blocks?
    pub eh_generation: u32, // generation of the tree
}


// https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4_extents.h
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ExtentIdx {
    pub ei_block: u32,   // index covers logical blocks from 'block'
    pub ei_leaf_lo: u32, // pointer to the physical block of the next
                         // level. leaf or next index could be there
    pub ei_leaf_hi: u16, // high 16 bits of physical block
    pub ei_unused: u16,
}


// https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4_extents.h
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ExtentTail {
    pub et_checksum: u32, // crc32c(uuid+inum+extent_block)
}


pub fn scan_extent_tree(
    map: &mut UsageMap,
    inode: &Inode,
    osd2: &Osd2,
    fs: &Fs,
    ctx: &mut Context,
) -> anyhow::Result<()>
{
    Ok(()) // TODO
}
