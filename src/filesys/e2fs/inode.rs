use std::io::{Read, Seek, SeekFrom};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use bincode::{DefaultOptions, Options};

use crate::Context;
use crate::usage_map::{UsageMap, AllocStatus};
use crate::hilo;

use crate::{
    bs,
    alloc_inode_size,
};
use super::{
    Fs,
    FsCreator,
    fetch_regular_bg_descriptor,
    extent::{
        self,
        ExtentTree,
        ExtentTreeIterator,
    },
};


// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h

pub const GOOD_OLD_INODE_SIZE: u16 = 128;
pub const N_BLOCKS: usize = 15;


/// Ext4 inode.
/// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Inode {
    pub i_mode: u16,              // File mode
    pub i_uid: u16,               // Low 16 bits of Owner Uid
    pub i_size_lo: u32,           // Size in bytes
    pub i_atime: u32,             // Access time
    pub i_ctime: u32,             // Inode Change time
    pub i_mtime: u32,             // Modification time
    pub i_dtime: u32,             // Deletion Time
    pub i_gid: u16,               // Low 16 bits of Group Id
    pub i_links_count: u16,       // Links count
    pub i_blocks_lo: u32,         // Blocks count
    pub i_flags: u32,             // File flags
    pub osd1: u32,                // OS dependent 1
    pub i_block: [u32; N_BLOCKS], // Pointers to blocks
    pub i_generation: u32,        // File version (for NFS)
    pub i_file_acl_lo: u32,       // File ACL
    pub i_size_high: u32,
    pub i_obso_faddr: u32,        // Obsoleted fragment address
    pub osd2: [u8; 12],           // OS dependent 2
    pub i_extra_isize: u16,
    pub i_checksum_hi: u16,       // crc32c(uuid+inum+inode) BE
    pub i_ctime_extra: u32,       // extra Change time      (nsec << 2 | epoch)
    pub i_mtime_extra: u32,       // extra Modification time(nsec << 2 | epoch)
    pub i_atime_extra: u32,       // extra Access time      (nsec << 2 | epoch)
    pub i_crtime: u32,            // File Creation time
    pub i_crtime_extra: u32,      // extra FileCreationtime (nsec << 2 | epoch)
    pub i_version_hi: u32,        // high 32 bits for 64-bit version
    pub i_projid: u32,            // Project ID
}


pub const INODE_STRUCT_SIZE: usize = 160;


// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h#L811
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Osd2Linux {
    pub l_i_blocks_high: u16, // were l_i_reserved1
    pub l_i_file_acl_high: u16,
    pub l_i_uid_high: u16,    // these 2 fields
    pub l_i_gid_high: u16,    // were reserved2[0]
    pub l_i_checksum_lo: u16, // crc32c(uuid+inum+inode) LE
    pub l_i_reserved: u16,
}


// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h#L811
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Osd2Hurd {
    pub h_i_reserved1: u16, // Obsoleted fragment number/size which are removed in ext4
    pub h_i_mode_high: u16,
    pub h_i_uid_high: u16,
    pub h_i_gid_high: u16,
    pub h_i_author: u32,
}


// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h#L811
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Osd2Masix {
    pub h_i_reserved1: u16,      // Obsoleted fragment number/size which are removed in ext4
    pub m_i_file_acl_high: u16,
    pub m_i_reserved2: [u32; 2],
}


/// Inode flags (i_flags)
struct IFlags(u32);

impl IFlags {
    pub fn has_secrm(&self)            -> bool { self.0 & 0x1 != 0 }
    pub fn has_unrm(&self)             -> bool { self.0 & 0x2 != 0 }
    pub fn has_compr(&self)            -> bool { self.0 & 0x4 != 0 }
    pub fn has_sync(&self)             -> bool { self.0 & 0x8 != 0 }
    pub fn has_immutable(&self)        -> bool { self.0 & 0x10 != 0 }
    pub fn has_append(&self)           -> bool { self.0 & 0x20 != 0 }
    pub fn has_nodump(&self)           -> bool { self.0 & 0x40 != 0 }
    pub fn has_noatime(&self)          -> bool { self.0 & 0x80 != 0 }
    pub fn has_dirty(&self)            -> bool { self.0 & 0x100 != 0 }
    pub fn has_comprblk(&self)         -> bool { self.0 & 0x200 != 0 }
    pub fn has_nocompr(&self)          -> bool { self.0 & 0x400 != 0 }
    pub fn has_encrypt(&self)          -> bool { self.0 & 0x800 != 0 }
    pub fn has_index(&self)            -> bool { self.0 & 0x1000 != 0 }
    pub fn has_imagic(&self)           -> bool { self.0 & 0x2000 != 0 }
    pub fn has_journal_data(&self)     -> bool { self.0 & 0x4000 != 0 }
    pub fn has_notail(&self)           -> bool { self.0 & 0x8000 != 0 }
    pub fn has_dirsync(&self)          -> bool { self.0 & 0x10000 != 0 }
    pub fn has_topdir(&self)           -> bool { self.0 & 0x20000 != 0 }
    pub fn has_huge_file(&self)        -> bool { self.0 & 0x40000 != 0 }
    pub fn has_extents(&self)          -> bool { self.0 & 0x80000 != 0 }
    pub fn has_verity(&self)           -> bool { self.0 & 0x100000 != 0 }
    pub fn has_ea_inode(&self)         -> bool { self.0 & 0x200000 != 0 }
    pub fn has_eofblocks(&self)        -> bool { self.0 & 0x400000 != 0 }
    // 0x800000 missing.
    pub fn has_snapfile(&self)         -> bool { self.0 & 0x1000000 != 0 }
    // 0x2000000 missing.
    pub fn has_snapfile_deleted(&self) -> bool { self.0 & 0x4000000 != 0 }
    pub fn has_snapfile_shrunk(&self)  -> bool { self.0 & 0x8000000 != 0 }
    pub fn has_inline_data(&self)      -> bool { self.0 & 0x10000000 != 0 }
    pub fn has_projinherit(&self)      -> bool { self.0 & 0x20000000 != 0 }
    // 0x40000000 missing.
    pub fn has_reserved(&self)         -> bool { self.0 & 0x80000000 != 0 }

    pub fn get_unknown(&self) -> u32
    {
        (self.0 & 0x800000) | (self.0 & 0x2000000) | (self.0 & 0x40000000)
    }

    pub fn has_unknown(&self) -> bool
    {
        self.get_unknown() != 0
    }
}


/// Inode mode (i_mode)
struct IMode(u16);

impl IMode {
    pub fn has_ixoth(&self)  -> bool { self.0 & 0x1 != 0}
    pub fn has_iwoth(&self)  -> bool { self.0 & 0x2 != 0}
    pub fn has_iroth(&self)  -> bool { self.0 & 0x4 != 0}
    pub fn has_ixgrp(&self)  -> bool { self.0 & 0x8 != 0}
    pub fn has_iwgrp(&self)  -> bool { self.0 & 0x10 != 0}
    pub fn has_irgrp(&self)  -> bool { self.0 & 0x20 != 0}
    pub fn has_ixusr(&self)  -> bool { self.0 & 0x40 != 0}
    pub fn has_iwusr(&self)  -> bool { self.0 & 0x80 != 0}
    pub fn has_irusr(&self)  -> bool { self.0 & 0x100 != 0}
    pub fn has_isvtx(&self)  -> bool { self.0 & 0x200 != 0}
    pub fn has_isgid(&self)  -> bool { self.0 & 0x400 != 0}
    pub fn has_isuid(&self)  -> bool { self.0 & 0x800 != 0}
    pub fn has_ififo(&self)  -> bool { self.0 & 0x1000 != 0}
    pub fn has_ifchr(&self)  -> bool { self.0 & 0x2000 != 0}
    pub fn has_ifdir(&self)  -> bool { self.0 & 0x4000 != 0}
    pub fn has_ifblk(&self)  -> bool { self.has_ifchr() && self.has_ifdir() }
    pub fn has_ifreg(&self)  -> bool { self.0 & 0x8000 != 0}
    pub fn has_iflnk(&self)  -> bool { self.has_ifchr() && self.has_ifreg() }
    pub fn has_ifsock(&self) -> bool { self.has_ifdir() && self.has_ifreg() }

    pub fn get_unknown(&self) -> u32 { 0 }

    pub fn has_unknown(&self) -> bool
    {
        self.get_unknown() != 0
    }
}


/// Osd2 structure (i_osd2)
#[derive(Clone, Debug)]
pub enum Osd2 {
    Linux(Osd2Linux),
    Hurd(Osd2Hurd),
    Masix(Osd2Masix),
}


/// Ext2 file types (plus some custom ones).
#[derive(Clone, Debug)]
enum InodeType {
    Fifo,
    Character,
    Directory,
    Block,
    Regular,
    SymLink,
    Socket,
    Ea,
    Journal,
}


/// Fetches an inode, based on the number of the inode.
pub fn fetch_inode(inum: u64, fs: &Fs, ctx: &mut Context) -> anyhow::Result<Inode>
{
    let bg_num = (inum - 1) / fs.sb.s_inodes_per_group as u64;
    let idx = (inum - 1) % fs.sb.s_inodes_per_group as u64;

    let mut itable = vec![
        u8::default();
        fs.sb.s_inodes_per_group as usize * alloc_inode_size!(fs.inode_size)
    ];
    read_itable(bg_num, &mut itable, fs, ctx)?;

    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    let inode: Inode = bincode_opt.deserialize(&itable[(idx * fs.inode_size) as usize..])?;

    Ok(inode)
}


/// Reads a group's raw inode table, into the supplied buffer.
pub fn read_itable(bg_num: u64, buf: &mut [u8], fs: &Fs, ctx: &mut Context) -> anyhow::Result<()>
{
    assert!(buf.len() >= fs.sb.s_inodes_per_group as usize * alloc_inode_size!(fs.inode_size));

    let desc = fetch_regular_bg_descriptor(bg_num, fs)?;
    let inode_table_block = if fs.opts.bit64_cfg.is_some() {
        hilo!(desc.bg_inode_table_hi, desc.bg_inode_table_lo)
    } else {
        desc.bg_inode_table_lo as u64
    };
    let offset = inode_table_block * bs!(fs.sb.s_log_block_size);

    ctx.drive.seek(SeekFrom::Start(offset))?;
    // FIXME: This could fail if the inode is smaller than INODE_STRUCT_SIZE and it is located at
    // the end of the disk. The read operation would then attempt to reach beyond the end of the
    // disk.
    ctx.drive.read_exact(buf)?;

    Ok(())
}


/// Scans an inode, specified by the index into  the supplied inode table.
pub fn scan_inode(
    map: &mut UsageMap,
    idx: usize,
    bg_num: u64,
    itable: &mut [u8],
    fs: &Fs,
    ctx: &mut Context,
) -> anyhow::Result<()>
{
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    let inode: Inode = bincode_opt.deserialize(&itable[idx * fs.inode_size as usize..])?;

    // NOTE: This is not tested. Linux is the only supported platform.
    let osd2 = match fs.opts.fs_creator {
        FsCreator::Hurd => Osd2::Hurd(bincode_opt.deserialize(&inode.osd2)?),
        FsCreator::Masix => Osd2::Masix(bincode_opt.deserialize(&inode.osd2)?),
        _ => Osd2::Linux(bincode_opt.deserialize(&inode.osd2)?),
    };
    let i_flags = IFlags { 0: inode.i_flags };

    println!("{}", idx); // [debug]
    println!("{:#?}", inode); // [debug]
    println!("{:#?}", osd2); // [debug]

    // Check inode flags.

    if i_flags.has_unknown() {
        bail!("inode {} has unknown flags: {:#10x}", idx, i_flags.get_unknown());
    } else if i_flags.has_encrypt() {
        bail!("inode {} has is encrypted", idx);
    } else if i_flags.has_imagic() {
        bail!("inode {} has an unsupported feature: imagic", idx);
    } else if i_flags.has_snapfile() {
        bail!("inode {} has an unsupported feature: snapfile", idx);
    } else if i_flags.has_snapfile_shrunk() {
        bail!("inode {} has an unsupported feature: snapfile_shrunk", idx);
    } else if i_flags.has_snapfile_deleted() {
        bail!("inode {} has an unsupported feature: snapfile_deleted", idx);
    } else if i_flags.has_compr() {
        bail!("inode {} has an unsupported feature: compr", idx);
    } else if i_flags.has_comprblk() {
        bail!("inode {} has an unsupported feature: comprblk", idx);
    }

    let i_mode = IMode { 0: inode.i_mode };

    let inode_type = if bg_num == 0 && idx + 1 == 8 {
        InodeType::Journal
    // NOTE: feature support is not checked.
    } else if i_flags.has_ea_inode() {
        InodeType::Ea
    } else if i_mode.has_ifsock() {
        InodeType::Socket
    } else if i_mode.has_iflnk() {
        InodeType::SymLink
    } else if i_mode.has_ifblk() {
        InodeType::Block
    } else if i_mode.has_ifreg() {
        InodeType::Regular
    } else if i_mode.has_ifdir() {
        InodeType::Directory
    } else if i_mode.has_ifchr() {
        InodeType::Character
    } else if i_mode.has_ififo() {
        InodeType::Fifo
    // Reserved inodes that are zeroed out.
    } else if bg_num == 0 && inode.i_mode == 0 && idx + 1 < fs.sb.s_first_ino as usize {
        println!("SKIPPED"); // [debug]
        return Ok(())
    } else {
        bail!("inode {} has invalid mode: {:x}", idx, inode.i_mode & 0xf000);
    };

    match inode_type {
        InodeType::Journal => scan_journal_iblock(map, &inode, &osd2, fs, ctx)?,
        InodeType::Ea => scan_ea_iblock(map, &inode, &osd2, fs, ctx)?,
        InodeType::Regular => scan_regular_iblock(map, &inode, &osd2, fs, ctx)?,
        InodeType::Directory => scan_dir_iblock(map, &inode, &osd2, fs, ctx)?,
        InodeType::SymLink => scan_symlink_iblock(map, &inode, &osd2, fs, ctx)?,
        // Undocumented special files are handled as regular files, just in case they use external
        // blocks.
        InodeType::Fifo |
        InodeType::Block |
        InodeType::Character |
        InodeType::Socket => scan_regular_iblock(map, &inode, &osd2, fs, ctx)?,
    }

    if i_flags.has_verity() {
        // TODO: verity
        bail!("inode {} has verity files", idx);
    }

    // TODO: xattrs
    // Possibly more ...

    Ok(()) // TODO
}


/// General-purpose procedure for scanning inode's i_block.
/// Used for regular files, symlinks, and other file types that do not require special handling.
fn scan_regular_iblock(
    map: &mut UsageMap,
    inode: &Inode,
    osd2: &Osd2,
    fs: &Fs,
    ctx: &mut Context
) -> anyhow::Result<()>
{
    let i_flags = IFlags { 0: inode.i_flags };

    // NOTE: Feature support is not being checked.
    // Inodes' i_flags fields are trusted.

    if i_flags.has_inline_data() {
        return Ok(());
    }

    let blocks = get_block_count(inode, osd2, fs);

    // Symlinks do not have inline_data flags set when inlined.
    // NOTE: don't have to check the file size here, as the only way for the block count to grow,
    // is verity. Fragments might be an issue, but those are obsolete and not secured by feature
    // flags It is assumed therefore, that they are a non-issue.
    if blocks == 0 {
        return Ok(());
    }

    let file_size = hilo!(inode.i_size_high, inode.i_size_lo);

    if i_flags.has_extents() {
        extent::scan_extent_tree(map, inode, fs, ctx)?;

        let extent_tree = ExtentTree::new(inode, fs, ctx)?;
        let extent_iterator = ExtentTreeIterator::new(&extent_tree); // [debug]

        println!("{:#?}", extent_tree); // [debug]

        for e in extent_iterator {
            println!("{:#?}", e); // [debug]

            // Position within the file.
            let log_start = e.ee_block as u64 * bs!(fs.sb.s_log_block_size);

            if log_start >= file_size {
                continue;
            }

            let mut len = e.ee_len as u64 * bs!(fs.sb.s_log_block_size);
            if log_start + len > file_size {
                len = file_size - log_start;
            }

            // Position on the disk.
            let start = hilo!(e.ee_start_hi, e.ee_start_lo) * bs!(fs.sb.s_log_block_size);

            println!("log_start: {}", log_start); // [debug]
            println!("len: {}", len); // [debug]
            println!("start: {}", start); // [debug]

            map.update(start, len, AllocStatus::Used);
        }
    } else {
        // The count of the block groups that were processed.
        let mut block_head = 0;

        // Scanning the 1st 12 direct blocks.
        for i in 0..12 {
            if block_head >=  blocks {
                break;
            }

            // Position within the file.
            let log_start = block_head * bs!(fs.sb.s_log_block_size);

            if log_start >= file_size {
                break;
            }

            let mut len = bs!(fs.sb.s_log_block_size);
            if log_start + len > file_size {
                len = file_size - log_start;
            }

            // Position on the disk.
            let start = inode.i_block[i] as u64 * bs!(fs.sb.s_log_block_size);

            // Skip null entries.
            if start == 0 {
                println!("direct block {} skipped", i); // [debug]
                continue;
            }

            println!("log_start: {}", log_start); // [debug]
            println!("len: {}", len); // [debug]
            println!("start: {}", start); // [debug]

            map.update(start, len, AllocStatus::Used);
            block_head += 1;
        }

        scan_indirect_block(map, &mut block_head, inode.i_block[12] as u64, inode, osd2, fs, ctx)?;
        scan_double_indirect_block(map, &mut block_head, inode.i_block[13] as u64, inode, osd2, fs, ctx)?;
        scan_triple_indirect_block(map, &mut block_head, inode.i_block[14] as u64, inode, osd2, fs, ctx)?;
    }

    Ok(())
}


fn scan_dir_iblock(_map: &mut UsageMap, _inode: &Inode, _osd2: &Osd2, _fs: &Fs, _ctx: &mut Context) -> anyhow::Result<()>
{
    Ok(()) // TODO
}


fn scan_symlink_iblock(map: &mut UsageMap, inode: &Inode, osd2: &Osd2, fs: &Fs, ctx: &mut Context) -> anyhow::Result<()>
{
    scan_regular_iblock(map, inode, osd2, fs, ctx)
}


fn scan_journal_iblock(_map: &mut UsageMap, _inode: &Inode, _osd2: &Osd2, _fs: &Fs, _ctx: &mut Context) -> anyhow::Result<()>
{
    Ok(()) // TODO
}


fn scan_ea_iblock(_map: &mut UsageMap, _inode: &Inode, _osd2: &Osd2, _fs: &Fs, _ctx: &mut Context) -> anyhow::Result<()>
{
    Ok(()) // TODO
}


fn scan_indirect_block(
    map: &mut UsageMap,
    block_head: &mut u64,
    block: u64,
    inode: &Inode,
    osd2: &Osd2,
    fs: &Fs,
    ctx: &mut Context
) -> anyhow::Result<()>
{
    // Check for a null block number.
    if block == 0 {
        return Ok(());
    }

    println!("scanning indirect block {}", block); // [debug]

    let block_address = block * bs!(fs.sb.s_log_block_size);
    let mut block_buf = vec![u8::default(); bs!(fs.sb.s_log_block_size) as usize];
    ctx.drive.seek(SeekFrom::Start(block_address))?;
    ctx.drive.read_exact(&mut block_buf)?;

    let mut entry_buf = <[u8; 4]>::default();
    let max_blocks = get_block_count(inode, osd2, fs);
    let file_size = hilo!(inode.i_size_high, inode.i_size_lo);
    let entries_in_a_block = bs!(fs.sb.s_log_block_size) as usize / 4;

    for i in 0..entries_in_a_block {
        if *block_head >= max_blocks {
            break;
        }

        // Position within the file.
        let log_start = *block_head * bs!(fs.sb.s_log_block_size);

        if log_start >= file_size {
            break;
        }

        let mut len = bs!(fs.sb.s_log_block_size);
        if log_start + len > file_size {
            len = file_size - log_start;
        }

        entry_buf[0] = block_buf[i * 4];
        entry_buf[1] = block_buf[i * 4 + 1];
        entry_buf[2] = block_buf[i * 4 + 2];
        entry_buf[3] = block_buf[i * 4 + 3];

        let start = u32::from_le_bytes(entry_buf) as u64 * bs!(fs.sb.s_log_block_size);

        // Check for null entries.
        if start == 0 {
            println!("indirect block {} entry {} skipped", block, i);
            continue;
        }

        println!("log_start: {}", log_start); // [debug]
        println!("len: {}", len); // [debug]
        println!("start: {}", start); // [debug]

        map.update(start, len, AllocStatus::Used);
        *block_head += 1;
    }

    Ok(())
}


fn scan_double_indirect_block(
    map: &mut UsageMap,
    block_head: &mut u64,
    block: u64,
    inode: &Inode,
    osd2: &Osd2,
    fs: &Fs,
    ctx: &mut Context
) -> anyhow::Result<()>
{
    // Check for a null block number.
    if block == 0 {
        return Ok(());
    }

    println!("scanning double indirect block {}", block); // [debug]

    let block_address = block * bs!(fs.sb.s_log_block_size);
    let mut block_buf = vec![u8::default(); bs!(fs.sb.s_log_block_size) as usize];
    ctx.drive.seek(SeekFrom::Start(block_address))?;
    ctx.drive.read_exact(&mut block_buf)?;

    let mut entry_buf = <[u8; 4]>::default();
    let max_blocks = get_block_count(inode, osd2, fs);
    let entries_in_a_block = bs!(fs.sb.s_log_block_size) as usize / 4;

    for i in 0..entries_in_a_block {
        if *block_head >= max_blocks {
            break;
        }

        entry_buf[0] = block_buf[i * 4];
        entry_buf[1] = block_buf[i * 4 + 1];
        entry_buf[2] = block_buf[i * 4 + 2];
        entry_buf[3] = block_buf[i * 4 + 3];

        let indirect_block = u32::from_le_bytes(entry_buf) as u64;

        // Check for null entries.
        if indirect_block == 0 {
            println!("double indirect block {} entry {} skipped", block, i);
            continue;
        }

        scan_indirect_block(map, block_head, indirect_block, inode, osd2, fs, ctx)?;
    }

    Ok(())
}


fn scan_triple_indirect_block(
    map: &mut UsageMap,
    block_head: &mut u64,
    block: u64,
    inode: &Inode,
    osd2: &Osd2,
    fs: &Fs,
    ctx: &mut Context
) -> anyhow::Result<()>
{
    // Check for a null block number.
    if block == 0 {
        return Ok(());
    }

    println!("scanning triple indirect block {}", block); // [debug]

    let block_address = block * bs!(fs.sb.s_log_block_size);
    let mut block_buf = vec![u8::default(); bs!(fs.sb.s_log_block_size) as usize];
    ctx.drive.seek(SeekFrom::Start(block_address))?;
    ctx.drive.read_exact(&mut block_buf)?;

    let mut entry_buf = <[u8; 4]>::default();
    let max_blocks = get_block_count(inode, osd2, fs);
    let entries_in_a_block = bs!(fs.sb.s_log_block_size) as usize / 4;

    for i in 0..entries_in_a_block {
        if *block_head >= max_blocks {
            break;
        }

        entry_buf[0] = block_buf[i * 4];
        entry_buf[1] = block_buf[i * 4 + 1];
        entry_buf[2] = block_buf[i * 4 + 2];
        entry_buf[3] = block_buf[i * 4 + 3];

        let double_indirect_block = u32::from_le_bytes(entry_buf) as u64;

        // Check for null entries.
        if double_indirect_block == 0 {
            println!("triple indirect block {} entry {} skipped", block, i);
            continue;
        }

        scan_double_indirect_block(map, block_head, double_indirect_block, inode, osd2, fs, ctx)?;
    }

    Ok(())
}


/// Returns the number of blocks occupied by the inode's data.
fn get_block_count(inode: &Inode, osd2: &Osd2, fs: &Fs) -> u64
{
    let i_flags = IFlags { 0: inode.i_flags };

    // The number of disk blocks.
    let mut blocks = inode.i_blocks_lo as u64;
    if let Some(dyn_cfg) = fs.opts.dyn_cfg {
        if dyn_cfg.ro_compat.has_huge_file() {
            if let Osd2::Linux(l) = osd2 {
                blocks = hilo!(l.l_i_blocks_high, inode.i_blocks_lo);
            }
        }
    }

    // Multiply by the size of the disk blocks.
    // This block sizing is unique to inodes only.
    blocks *= if i_flags.has_huge_file() {
        bs!(fs.sb.s_log_block_size)
    } else {
        512
    };
    // Divide by the size of the file system blocks.
    // FIXME: remove this check.
    assert!(blocks % bs!(fs.sb.s_log_block_size) == 0);
    blocks /= bs!(fs.sb.s_log_block_size);

    blocks
}
