#![allow(dead_code)]
use std::io::{Read, Seek, SeekFrom};
use anyhow::bail;
use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};

use crate::array::Array;
use crate::bitmap::Bitmap;
use crate::fill;
use crate::hilo;
use crate::usage_map::{AllocStatus, UsageMap};
use crate::{Config, Context};

mod extent;
mod inode;
#[macro_use]
mod macros;

use inode::{
    INODE_STRUCT_SIZE,
    GOOD_OLD_INODE_SIZE,
};


/// The Ext2/3/4 Superblock structure.
/// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuperBlock {
    pub s_inodes_count: u32,         // Inodes count
    pub s_blocks_count_lo: u32,      // Blocks count
    pub s_r_blocks_count_lo: u32,    // Reserved blocks count
    pub s_free_blocks_count_lo: u32, // Free blocks count
    pub s_free_inodes_count: u32,    // Free inodes count
    pub s_first_data_block: u32,     // First Data Block
    pub s_log_block_size: u32,       // Block size
    pub s_log_cluster_size: u32,     // Allocation cluster size
    pub s_blocks_per_group: u32,     // # Blocks per group
    pub s_clusters_per_group: u32,   // # Clusters per group
    pub s_inodes_per_group: u32,     // # Inodes per group
    pub s_mtime: u32,                // Mount time
    pub s_wtime: u32,                // Write time
    pub s_mnt_count: u16,            // Mount count
    pub s_max_mnt_count: u16,        // Maximal mount count
    pub s_magic: u16,                // Magic signature
    pub s_state: u16,                // File system state
    pub s_errors: u16,               // Behaviour when detecting errors
    pub s_minor_rev_level: u16,      // minor revision level
    pub s_lastcheck: u32,            // time of last check
    pub s_checkinterval: u32,        // max. time between checks
    pub s_creator_os: u32,           // OS
    pub s_rev_level: u32,            // Revision level
    pub s_def_resuid: u16,           // Default uid for reserved blocks
    pub s_def_resgid: u16,           // Default gid for reserved blocks
    // --- EXT4_DYNAMIC_REV ---
    pub s_first_ino: u32,         // First non-reserved inode
    pub s_inode_size: u16,        // size of inode structure
    pub s_block_group_nr: u16,    // block group # of this superblock
    pub s_feature_compat: u32,    // compatible feature set
    pub s_feature_incompat: u32,  // incompatible feature set
    pub s_feature_ro_compat: u32, // readonly-compatible feature set
    pub s_uuid: [u8; 16],         // 128-bit uuid for volume
    /// Type char[16].
    pub s_volume_name: [u8; 16], // volume name
    /// Type __nonstring char[64].
    pub s_last_mounted: Array<u8, 64>, // directory where last mounted
    pub s_algorithm_usage_bitmap: u32, // For compression
    // --- EXT4_FEATURE_COMPAT_DIR_PREALLOC ---
    pub s_prealloc_blocks: u8,     // Nr of blocks to try to preallocat
    pub s_prealloc_dir_blocks: u8, // Nr to preallocate for dirs
    /// Named `s_padding1` in Ext2.
    pub s_reserved_gdt_blocks: u16, // Per group desc for online growth
    // --- End of Ext2 superblock ---
    // --- EXT4_FEATURE_COMPAT_HAS_JOURNAL ---
    pub s_journal_uuid: [u8; 16], // uuid of journal superblock
    pub s_journal_inum: u32,      // inode number of journal file
    pub s_journal_dev: u32,       // device number of journal file
    pub s_last_orphan: u32,       // start of list of inodes to delete
    pub s_hash_seed: [u32; 4],    // HTREE hash seed
    pub s_def_hash_version: u8,   // Default hash version to use
    /// Named `s_reserved_char_pad` in Ext3.
    pub s_jnl_backup_type: u8,
    /// Named `s_reserved_word_pad` in Ext3.
    pub s_desc_size: u16, // size of group descriptor
    pub s_default_mount_opts: u32,
    pub s_first_meta_bg: u32, // First metablock block group
    // --- End of Ext3 superblock ---
    pub s_mkfs_time: u32,        // When the filesystem was created
    pub s_jnl_blocks: [u32; 17], // Backup of the journal inode
    // --- EXT4_FEATURE_COMPAT_64BIT ---
    pub s_blocks_count_hi: u32,         // Blocks count
    pub s_r_blocks_count_hi: u32,       // Reserved blocks count
    pub s_free_blocks_count_hi: u32,    // Free blocks count
    pub s_min_extra_isize: u16,         // All inodes have at least # bytes
    pub s_want_extra_isize: u16,        // New inodes should reserve # bytes
    pub s_flags: u32,                   // Miscellaneous flags
    pub s_raid_stride: u16,             // RAID stride
    pub s_mmp_update_interval: u16,     // # seconds to wait in MMP checking
    pub s_mmp_block: u64,               // Block for multi-mount protection
    pub s_raid_stripe_width: u32,       // blocks on all data disks (
    pub s_log_groups_per_flex: u8,      // FLEX_BG group size
    pub s_checksum_type: u8,            // metadata checksum algorithm used
    pub s_encryption_level: u8,         // versioning level for encryption
    pub s_reserved_pad: u8,             // Padding to next 32bits
    pub s_kbytes_written: u64,          // nr of lifetime kilobytes written
    pub s_snapshot_inum: u32,           // Inode number of active snapshot
    pub s_snapshot_id: u32,             // sequential ID of active snapshot
    pub s_snapshot_r_blocks_count: u64, // reserved blocks for active snapshot's future use
    pub s_snapshot_list: u32,           // inode number of the head of the on-disk snapshot list
    pub s_error_count: u32,             // number of fs errors
    pub s_first_error_time: u32,        // first time an error happened
    pub s_first_error_ino: u32,         // inode involved in first error
    pub s_first_error_block: u64,       // block involved of first error
    /// Type __nonstring __u8[32].
    pub s_first_error_func: [u8; 32], // function where the error happened
    pub s_first_error_line: u32,        // line number where error happened
    pub s_last_error_time: u32,         // most recent time of an error
    pub s_last_error_ino: u32,          // inode involved in last error
    pub s_last_error_line: u32,         // line number where error happened
    pub s_last_error_block: u64,        // block involved of last error
    /// Type __nonstring __u8[32].
    pub s_last_error_func: [u8; 32], // function where the error happened
    pub s_mount_opts: Array<u8, 64>,
    pub s_usr_quota_inum: u32,       // inode for tracking user quota
    pub s_grp_quota_inum: u32,       // inode for tracking group quota
    pub s_overhead_clusters: u32,    // overhead blocks/clusters in fs
    pub s_backup_bgs: [u32; 2],      // groups with sparse_super2 SBs
    pub s_encrypt_algos: [u8; 4],    // Encryption algorithms in use
    pub s_encrypt_pw_salt: [u8; 16], // Salt used for string2key algorithm
    pub s_lpf_ino: u32,              // Location of the lost+found inode
    pub s_prj_quota_inum: u32,       // inode for tracking project quota
    pub s_checksum_seed: u32,        // crc32c(uuid) if csum_seed set
    pub s_wtime_hi: u8,
    pub s_mtime_hi: u8,
    pub s_mkfs_time_hi: u8,
    pub s_lastcheck_hi: u8,
    pub s_first_error_time_hi: u8,
    pub s_last_error_time_hi: u8,
    pub s_first_error_errcode: u8,
    pub s_last_error_errcode: u8,
    pub s_encoding: u16,            // Filename charset encoding
    pub s_encoding_flags: u16,      // Filename charset encoding flags
    pub s_orphan_file_inum: u32,    // Inode for tracking orphan inodes
    pub s_reserved: Array<u32, 94>, // Padding to the end of the block
    pub s_checksum: u32,            // crc32c(superblock)
}


/// The Ext2/3/4 group descriptor structure.
/// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupDescriptor {
    pub bg_block_bitmap_lo: u32,      // Blocks bitmap block
    pub bg_inode_bitmap_lo: u32,      // Inodes bitmap block
    pub bg_inode_table_lo: u32,       // Inodes table block
    pub bg_free_blocks_count_lo: u16, // Free blocks count
    pub bg_free_inodes_count_lo: u16, // Free inodes count
    pub bg_used_dirs_count_lo: u16,   // Directories count
    pub bg_flags: u16,                // EXT4_BG_flags (INODE_UNINIT, etc)
    pub bg_exclude_bitmap_lo: u32,    // Exclude bitmap for snapshots
    pub bg_block_bitmap_csum_lo: u16, // crc32c(s_uuid+grp_num+bbitmap) LE
    pub bg_inode_bitmap_csum_lo: u16, // crc32c(s_uuid+grp_num+ibitmap) LE
    pub bg_itable_unused_lo: u16,     // Unused inodes count
    pub bg_checksum: u16,             // crc16(sb_uuid+group+desc)
    // --- End of Ext2/3 descriptor ---
    pub bg_block_bitmap_hi: u32,      // Blocks bitmap block MSB
    pub bg_inode_bitmap_hi: u32,      // Inodes bitmap block MSB
    pub bg_inode_table_hi: u32,       // Inodes table block MSB
    pub bg_free_blocks_count_hi: u16, // Free blocks count MSB
    pub bg_free_inodes_count_hi: u16, // Free inodes count MSB
    pub bg_used_dirs_count_hi: u16,   // Directories count MSB
    pub bg_itable_unused_hi: u16,     // Unused inodes count MSB
    pub bg_exclude_bitmap_hi: u32,    // Exclude bitmap block MSB
    pub bg_block_bitmap_csum_hi: u16, // crc32c(s_uuid+grp_num+bbitmap) BE
    pub bg_inode_bitmap_csum_hi: u16, // crc32c(s_uuid+grp_num+ibitmap) BE
    pub bg_reserved: u32,
}


const GROUP_DESC_STRUCT_SIZE: usize = 64;


// FIXME: Debug is derived.
/// Group descriptor flags (bg_flags).
#[derive(Copy, Clone, Debug)]
pub struct BgFlags(u16);

impl BgFlags {
    pub fn has_inode_uninit(&self) -> bool { self.0 & 0x1 != 0 }
    pub fn has_block_uninit(&self) -> bool { self.0 & 0x2 != 0 }
    pub fn has_inode_zeroed(&self) -> bool { self.0 & 0x4 != 0 }

    pub fn get_unknown(&self) -> u16
    {
        (self.0 >> 3) << 3
    }

    pub fn has_unknown(&self) -> bool
    {
        self.get_unknown() != 0
    }
}


/// State of the f&selfile system (s_state).
#[derive(Copy, Clone)]
pub struct State(u16);

impl State {
    pub fn has_valid(&self)     -> bool { self.0 & 0x1 != 0 }
    pub fn has_error(&self)     -> bool { self.0 & 0x2 != 0 }
    pub fn has_orphan(&self)    -> bool { self.0 & 0x4 != 0 }
    /// Not refere&selfnced in the documentation.
    pub fn has_fc_replay(&self) -> bool { self.0 & 0x20 != 0 }

    pub fn get_unknown(&self) -> u16
    {
        ((self.0 >> 6) << 6) | (self.0 & 0x08) | (self.0 & 0x10)
    }

    pub fn has_unknown(&self) -> bool
    {
        self.get_unknown() != 0
    }
}


/// Error policy (s_errors).
#[derive(Copy, Clone, Debug)]
pub enum ErrorPolicy {
    /// Invalid value. `s_errors` == 0.
    Null,
    Continue,
    ReadOnly,
    Panic,
}


/// File system creator OS.
#[derive(Copy, Clone, Debug)]
pub enum FsCreator {
    Linux,
    Hurd,
    Masix,
    FreeBSD,
    Lites,
}


/// Revision level (s_rev_level).
#[derive(Copy, Clone, Debug)]
pub enum Revision {
    GoodOld,
    Dynamic,
}


/// Compatible features (s_feature_compat).
#[derive(Copy, Clone)]
pub struct CompatFeatures(u32);

impl CompatFeatures {
    pub fn has_dir_prealloc(&self)   -> bool { self.0 & 0x0001 != 0 }
    pub fn has_imagic_inodes(&self)  -> bool { self.0 & 0x0002 != 0 }
    pub fn has_has_journal(&self)    -> bool { self.0 & 0x0004 != 0 }
    pub fn has_ext_attr(&self)       -> bool { self.0 & 0x0008 != 0 }
    pub fn has_resize_inode(&self)   -> bool { self.0 & 0x0010 != 0 }
    pub fn has_dir_index(&self)      -> bool { self.0 & 0x0020 != 0 }
    /// Not used in the kernel.
    pub fn has_lazy_bg(&self)        -> bool { self.0 & 0x0040 != 0 }
    /// Not used in the kernel, neither in e2fsprogs.
    pub fn has_exclude_inode(&self)  -> bool { self.0 & 0x0080 != 0 }
    /// Not used in the kernel.
    pub fn has_exclude_bitmap(&self) -> bool { self.0 & 0x0100 != 0 }
    pub fn has_sparse_super2(&self)  -> bool { self.0 & 0x0200 != 0 }
    pub fn has_fast_commit(&self)    -> bool { self.0 & 0x0400 != 0 }
    /// Not referenced in the documentation.
    pub fn has_stable_inodes(&self)  -> bool { self.0 & 0x0800 != 0 }
    pub fn has_orphan_file(&self)    -> bool { self.0 & 0x1000 != 0 }

    pub fn get_unknown(&self) -> u32
    {
        (self.0 >> 13) << 13
    }

    pub fn has_unknown(&self) -> bool
    {
        self.get_unknown() != 0
    }
}


/// Incompatible features (s_feature_incompat).
#[derive(Copy, Clone)]
pub struct IncompatFeatures(u32);

impl IncompatFeatures {
    pub fn has_compression(&self) -> bool { self.0 & 0x00001 != 0 }
    pub fn has_filetype(&self)    -> bool { self.0 & 0x00002 != 0 }
    pub fn has_recover(&self)     -> bool { self.0 & 0x00004 != 0 }
    pub fn has_journal_dev(&self) -> bool { self.0 & 0x00008 != 0 }
    pub fn has_meta_bg(&self)     -> bool { self.0 & 0x00010 != 0 }
    // 0x00020 missing.
    pub fn has_extents(&self)     -> bool { self.0 & 0x00040 != 0 }
    pub fn has_64bit(&self)       -> bool { self.0 & 0x00080 != 0 }
    pub fn has_mmp(&self)         -> bool { self.0 & 0x00100 != 0 }
    pub fn has_flex_bg(&self)     -> bool { self.0 & 0x00200 != 0 }
    pub fn has_ea_inode(&self)    -> bool { self.0 & 0x00400 != 0 }
    // 0x0800 missing.
    pub fn has_dirdata(&self)     -> bool { self.0 & 0x01000 != 0 }
    pub fn has_csum_seed(&self)   -> bool { self.0 & 0x02000 != 0 }
    pub fn has_largedir(&self)    -> bool { self.0 & 0x04000 != 0 }
    pub fn has_inline_data(&self) -> bool { self.0 & 0x08000 != 0 }
    pub fn has_encrypt(&self)     -> bool { self.0 & 0x10000 != 0 }
    /// Not referenced in the documentation.
    pub fn has_casefold(&self)    -> bool { self.0 & 0x20000 != 0 }

    pub fn get_unknown(&self) -> u32
    {
        ((self.0 >> 18) << 18) | (self.0 & 0x00020) | (self.0 & 0x00800)
    }

    pub fn has_unknown(&self) -> bool
    {
        self.get_unknown() != 0
    }
}


/// Read-only compatible features (s_feature_ro_compat).
#[derive(Copy, Clone)]
pub struct RoCompatFeatures(u32);

impl RoCompatFeatures {
    pub fn has_sparse_super(&self)   -> bool { self.0 & 0x00001 != 0 }
    pub fn has_large_file(&self)     -> bool { self.0 & 0x00002 != 0 }
    /// Not used in e2fsprogs.
    pub fn has_btree_dir(&self)      -> bool { self.0 & 0x00004 != 0 }
    pub fn has_huge_file(&self)      -> bool { self.0 & 0x00008 != 0 }
    pub fn has_gdt_csum(&self)       -> bool { self.0 & 0x00010 != 0 }
    pub fn has_dir_nlink(&self)      -> bool { self.0 & 0x00020 != 0 }
    pub fn has_extra_isize(&self)    -> bool { self.0 & 0x00040 != 0 }
    /// Not used in the kernel, neither in e2fsprogs.
    pub fn has_has_snapshot(&self)   -> bool { self.0 & 0x00080 != 0 }
    pub fn has_quota(&self)          -> bool { self.0 & 0x00100 != 0 }
    pub fn has_bigalloc(&self)       -> bool { self.0 & 0x00200 != 0 }
    pub fn has_metadata_csum(&self)  -> bool { self.0 & 0x00400 != 0 }
    /// Not used in the kernel, neither in e2fsprogs.
    pub fn has_replica(&self)        -> bool { self.0 & 0x00800 != 0 }
    pub fn has_readonly(&self)       -> bool { self.0 & 0x01000 != 0 }
    pub fn has_project(&self)        -> bool { self.0 & 0x02000 != 0 }
    /// Not used in the kernel; not referenced in the documentation.
    pub fn has_shared_blocks(&self)  -> bool { self.0 & 0x04000 != 0 }
    pub fn has_verity(&self)         -> bool { self.0 & 0x08000 != 0 }
    pub fn has_orphan_present(&self) -> bool { self.0 & 0x10000 != 0 }

    pub fn get_unknown(&self) -> u32
    {
        (self.0 >> 17) << 17
    }

    pub fn has_unknown(&self) -> bool
    {
        self.get_unknown() != 0
    }
}


/// Hash versions.
#[derive(Copy, Clone, Debug)]
pub enum HashVersion {
    Legacy,
    HalfMD4,
    Tea,
    LegacyUnsigned,
    HalfMD4Unsigned,
    TeaUnsigned,
    /// It is not clear whether this should be in the default hash version.
    /// The documentation does not include it there.
    /// It does however mention it later in different structures.
    SipHash,
}


/// Default mount options (s_default_mount_opts).
#[derive(Copy, Clone)]
pub struct DefMountOpts(u32);

impl DefMountOpts {
    pub fn has_debug(&self)          -> bool { self.0 & 0x001 != 0 }
    pub fn has_bsdgroups(&self)      -> bool { self.0 & 0x002 != 0 }
    pub fn has_xattr_user(&self)     -> bool { self.0 & 0x004 != 0 }
    pub fn has_acl(&self)            -> bool { self.0 & 0x008 != 0 }
    pub fn has_uid16(&self)          -> bool { self.0 & 0x010 != 0 }
    pub fn has_jmode_data(&self)     -> bool { self.0 & 0x020 != 0 }
    pub fn has_jmode_ordered(&self)  -> bool { self.0 & 0x040 != 0 }
    pub fn has_jmode(&self)          -> bool { self.has_jmode_data() && self.has_jmode_ordered() }
    /// Not referenced in the documentation.
    /// Understandably so, as it is just an alias.
    pub fn has_jmode_wback(&self)    -> bool { self.has_jmode() }
    // 0x080 missing.
    pub fn has_nobarrier(&self)      -> bool { self.0 & 0x100 != 0 }
    pub fn has_block_validity(&self) -> bool { self.0 & 0x200 != 0 }
    pub fn has_discard(&self)        -> bool { self.0 & 0x400 != 0 }
    pub fn has_nodealloc(&self)      -> bool { self.0 & 0x800 != 0 }

    pub fn get_unknown(&self) -> u32
    {
        ((self.0 >> 14) << 14) | (self.0 & 0x080)
    }

    pub fn has_unknown(&self) -> bool
    {
        self.get_unknown() != 0
    }
}


/// Superblock flags (s_flags).
#[derive(Copy, Clone)]
pub struct Flags(u32);

impl Flags {
    pub fn has_signed_hash(&self)   -> bool { self.0 & 0x01 != 0 }
    pub fn has_unsigned_hash(&self) -> bool { self.0 & 0x02 != 0 }
    pub fn has_test(&self) -> bool { self.0 & 0x04 != 0 }
    // 0x08 missing.
    /// Not referenced in the documentation.
    pub fn has_is_snapshot(&self)   -> bool { self.0 & 0x10 != 0 }
    /// Not referenced in the documentation.
    pub fn has_fix_snapshot(&self)  -> bool { self.0 & 0x20 != 0 }
    /// Not referenced in the documentation.
    pub fn has_fix_exclude(&self)   -> bool { self.0 & 0x40 != 0 }

    pub fn get_unknown(&self) -> u32
    {
        ((self.0 >> 7) << 7) | (self.0 & 0x08)
    }

    pub fn has_unknown(&self) -> bool
    {
        self.get_unknown() != 0
    }
}


/// Encryption algorithms.
#[derive(Copy, Clone, Debug)]
pub enum EncryptAlgo {
    Null,
    AES256XTS,
    AES256GCM,
    AES256CBC,
    /// The documentation does not mention this.
    AES256CTS,
}


impl Default for EncryptAlgo {
    fn default() -> Self
    {
        Self::Null
    }
}


/// Filesystem parameters.
/// This structure contains all the relevant information about the filesystem. This includes
/// important structures and decoded values.
#[derive(Clone, Debug)]
pub struct Fs {
    pub sb: SuperBlock,
    pub desc_table: Vec<u8>,
    pub opts: FsOptions,
    // -- computed values --
    pub bg_count: u64,
    pub bg_size: u64,
    pub desc_size: u64,
    pub inode_size: u64,
    pub csum_seed: Option<u32>,
}


/// Decoded file system flag fields and enumerations; after validating all the options.
/// Contains all the flag fields and enumerations. Does not substitute, but complements the
/// SuperBlock structure.
#[derive(Copy, Clone, Debug)]
pub struct FsOptions {
    pub state: State,
    pub error_policy: ErrorPolicy,
    pub fs_creator: FsCreator,
    pub revision: Revision,
    pub dyn_cfg: Option<DynConfig>,
    pub journal_cfg: Option<JournalConfig>,
    pub bit64_cfg: Option<Bit64Config>,
}


/// Dynamic revision configuration.
#[derive(Copy, Clone, Debug)]
pub struct DynConfig {
    pub compat: CompatFeatures,
    pub incompat: IncompatFeatures,
    pub ro_compat: RoCompatFeatures,
}


/// Configuration for systems with journaling support.
#[derive(Copy, Clone, Debug)]
pub struct JournalConfig {
    pub def_hash_ver: HashVersion,
    pub def_mount_opts: DefMountOpts,
}


/// 64-bit configuration.
#[derive(Copy, Clone, Debug)]
pub struct Bit64Config {
    pub flags: Flags,
    pub encrypt_algos: Option<[EncryptAlgo; 4]>,
}


/// Process an Ext2/3/4 file system.
pub fn process_drive(ctx: &mut Context, cfg: &Config) -> anyhow::Result<()> {
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    ctx.drive.seek(SeekFrom::Start(1024))?;
    let sb: SuperBlock = bincode_opt.deserialize_from(&ctx.drive)?;
    let opts = get_and_check_fs_options(&sb, cfg)?;

    // Computing values that will be needed across multiple procedures.

    let blocks_count = if opts.bit64_cfg.is_some() {
        hilo!(sb.s_blocks_count_hi, sb.s_blocks_count_lo)
    } else {
        sb.s_blocks_count_lo as u64
    };
    let mut bg_count = (blocks_count - sb.s_first_data_block as u64) / sb.s_blocks_per_group as u64;
    if (blocks_count - sb.s_first_data_block as u64) % sb.s_blocks_per_group as u64 != 0 {
        bg_count += 1;
    }
    let bg_size = sb.s_blocks_per_group as u64 * bs!(sb.s_log_block_size);
    let desc_size = if sb.s_desc_size == 0 {
        32
    } else {
        sb.s_desc_size as u64
    };
    let inode_size = if opts.dyn_cfg.is_some() {
        sb.s_inode_size as u64
    } else {
        GOOD_OLD_INODE_SIZE as u64
    };
    // Source: https://github.com/tytso/e2fsprogs/blob/master/lib/ext2fs/csum.c#L33
    let csum_seed = if let Some(dyn_cfg) = opts.dyn_cfg {
        if dyn_cfg.incompat.has_csum_seed() {
            Some(sb.s_checksum_seed)
        } else if dyn_cfg.ro_compat.has_metadata_csum() || dyn_cfg.incompat.has_ea_inode() {
            Some(ext4_style_crc32c_le(!0, &sb.s_uuid))
        } else {
            None
        }
    } else {
        None
    };

    // Reading the group descriptor table from the disk.

    let mut desc_table = vec![
        u8::default();
        bg_count as usize * alloc_desc_size!(desc_size)
    ];
    ctx.drive.seek(SeekFrom::Start(start_of_first_gdt(&sb)))?;
    // FIXME: This could fail if the descriptor is smaller than GROUP_DESC_STRUCT_SIZE and it is
    // located at the end of the disk. The read operation would then attempt to reach beyond the
    // end of the disk.
    ctx.drive.read_exact(&mut desc_table)?;

    let fs = Fs {
        sb,
        desc_table,
        opts,
        bg_count,
        bg_size,
        desc_size,
        inode_size,
        csum_seed,
    };

    //println!("{:#?}", &fs); // [debug]

    //for i in 0..bg_count { // [debug]
    //let desc = fetch_regular_bg_descriptor(i, &fs)?; // [debug]
    //print!("{:04}: ", i); // [debug]
    //println!("{:#?}", &desc); // [debug]
    //if desc.bg_flags & 4 == 0 { // [debug]
    //println!("NOT ZEROED") // [debug]
    //} // [debug]
    //} // [debug]

    let free_blocks = scan_free_space(&fs, ctx, cfg)?;

    //println!("{:#?}", free_blocks); // [debug]

    if !cfg.report_only {
        fill::fill_free_space(&free_blocks, ctx, cfg)?;
    }

    Ok(())
}


/// Scans the drive for free space and returns a map of the usage.
fn scan_free_space(fs: &Fs, ctx: &mut Context, _cfg: &Config) -> anyhow::Result<UsageMap>
{
    let drive_size = ctx.drive.seek(SeekFrom::End(0))?;
    let mut map = UsageMap::new(drive_size);

    for num in 0..fs.bg_count {
        scan_regular_bg(&mut map, num, fs, ctx)?;
    }

    Ok(map)
}


/// Processes a regular block group, scans the free space and updates the supplied UsageMap.
fn scan_regular_bg(map: &mut UsageMap, bg_num: u64, fs: &Fs, ctx: &mut Context) -> anyhow::Result<()>
{
    ctx.logger.log(2, &format!("processing block group {:010}", bg_num));

    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    let block_size = bs!(fs.sb.s_log_block_size);
    let bg_start = start_of_bg(bg_num, fs);
    let has_csum = match fs.opts.dyn_cfg {
        Some(dyn_cfg) => dyn_cfg.ro_compat.has_metadata_csum() || dyn_cfg.ro_compat.has_gdt_csum(),
        None => false,
    };
    let mut skip_super = false;

    // Check if we skip the superblock and gdt.
    if let Some(dyn_cfg) = fs.opts.dyn_cfg {
        // Sparse_super2 is more aggressive, so we need to check that first.
        if dyn_cfg.compat.has_sparse_super2() {
            if bg_num != fs.sb.s_backup_bgs[0] as u64
                && bg_num != fs.sb.s_backup_bgs[1] as u64
            {
                skip_super = true;
            }
        } else if dyn_cfg.ro_compat.has_sparse_super()
            && bg_num != 0
            && bg_num % 3 != 0
            && bg_num % 5 != 0
            && bg_num % 7 != 0
        {
            skip_super = true;
        }
    }

    if !skip_super {
        let gdt_start: u64;

        // The superblock.
        if bg_num == 0 {
            // The empty space at the beginning of the drive and the superblock.
            map.update(0, 2048, AllocStatus::Used);
            gdt_start = start_of_first_gdt(&fs.sb);
        } else {
            map.update(bg_start, 1024, AllocStatus::Used);
            gdt_start = bg_start + block_size;
        }

        //println!("gdt start: {}", gdt_start); // [debug]

        // The group descriptors.
        if has_csum {
            // Read in this group's copy of the gdt.

            let mut gdt = vec![
                u8::default();
                fs.bg_count as usize * alloc_desc_size!(fs.desc_size)
            ];
            ctx.drive.seek(SeekFrom::Start(gdt_start))?;
            // FIXME: This could fail if the descriptor is smaller than GROUP_DESC_STRUCT_SIZE and
            // it is located at the end of the disk. The read operation would then attempt to reach
            // beyond the end of the disk.
            ctx.drive.read_exact(&mut gdt)?;

            for i in 0..fs.bg_count {
                let desc: GroupDescriptor =
                    bincode_opt.deserialize(&gdt[(i * fs.desc_size) as usize..])?;

                if verify_desc_csum(&desc, i, fs)? {
                    map.update(
                        gdt_start + (i * fs.desc_size),
                        fs.desc_size,
                        AllocStatus::Used,
                    );

                    //if i == 0 { // [debug]
                    //println!("verified"); // [debug]
                    //} // [debug]
                }
            }
        } else {
            // Without checksumming, the whole descriptor table must be initialised.
            map.update(gdt_start, fs.bg_count * fs.desc_size, AllocStatus::Used);
        }
    }

    let desc = fetch_regular_bg_descriptor(bg_num, fs)?;

    if has_csum {
        if !verify_desc_csum(&desc, bg_num, fs)? {
            ctx.logger.log(2, &format!("group descriptor {} has invalid checksum", bg_num));
            return Ok(());
        }
    }

    let bg_flags = BgFlags { 0: desc.bg_flags };

    if bg_flags.has_unknown() {
        ctx.logger.log(0, &format!("group descriptor {} has unknown flags: {}", bg_num, bg_flags.get_unknown()));
        bail!("{:?}", desc);
    }

    let inode_bitmap_block = if fs.opts.bit64_cfg.is_some() {
        hilo!(desc.bg_inode_bitmap_hi, desc.bg_inode_bitmap_lo)
    } else {
        desc.bg_inode_bitmap_lo as u64
    };

    //println!("inode bitmap: {}", inode_bitmap_block); // [debug]

    // Inode bitmap.
    if !bg_flags.has_inode_uninit() {
        map.update(
            inode_bitmap_block * block_size,
            block_size,
            AllocStatus::Used,
        );
    }

    let block_bitmap_block = if fs.opts.bit64_cfg.is_some() {
        hilo!(desc.bg_block_bitmap_hi, desc.bg_block_bitmap_lo)
    } else {
        desc.bg_block_bitmap_lo as u64
    };

    //println!("block bitmap: {}", block_bitmap_block); // [debug]

    // Block bitmap.
    if !bg_flags.has_block_uninit() {
        map.update(
            block_bitmap_block * block_size,
            block_size,
            AllocStatus::Used,
        );
    }

    let inode_table_block = if fs.opts.bit64_cfg.is_some() {
        hilo!(desc.bg_inode_table_hi, desc.bg_inode_table_lo)
    } else {
        desc.bg_inode_table_lo as u64
    };

    //println!("inode table: {}", inode_table_block); // [debug]

    // Inode table.
    if bg_flags.has_inode_zeroed() {
        map.update(
            inode_table_block * block_size,
            fs.sb.s_inodes_per_group as u64 * fs.inode_size,
            AllocStatus::Used,
        );
    } else if !bg_flags.has_inode_uninit() {
        // TODO: In the case where both inode_zeroed and inode_uninit flags are not present, the
        // inode table needs to be filled inode-by-inode, according to the inode bitmap.
        bail!("non-zeroed, but used, inode tables are not supported yet")
    }

    // Processing the inodes.

    if !bg_flags.has_inode_uninit() {
        ctx.drive.seek(SeekFrom::Start(inode_bitmap_block * block_size))?;
        let i_bmp = Bitmap::from_reader(&mut ctx.drive, block_size as usize)?;

        println!("{}", i_bmp); // [debug]

        let mut itable = vec![
            u8::default();
            fs.sb.s_inodes_per_group as usize * alloc_inode_size!(fs.inode_size)
        ];
        inode::read_itable(bg_num, &mut itable, fs, ctx)?;

        for i in 0..fs.sb.s_inodes_per_group as usize {
            if i_bmp.check_bit(i) {
                inode::scan_inode(map, i, bg_num, &mut itable, fs, ctx)?;
            }
        }
    }

    //ctx.drive.seek(SeekFrom::Start(block_bitmap_block * block_size))?;
    //let bmp = Bitmap::from_reader(&mut ctx.drive, block_size as usize)?;

    //let cluster_size = bs!(fs.sb.s_log_cluster_size);
    //let mut cluster_count = fs.sb.s_clusters_per_group as u64;

    //if bg_start + cluster_count * cluster_size > map.size() {
    //    let group_size = map.size() - bg_start;

    //    cluster_count = group_size / cluster_size;
    //    if group_size % cluster_size != 0 {
    //        cluster_count += 1;
    //    }
    //}

    //println!("cluster count: {}", cluster_count); // [debug]

    //// NOTE: When a block is marked as used, it does not necessarily mean that it is initialised.

    //if !bg_flags.has_block_uninit() {
    //    for i in 0..cluster_count {
    //        if bmp.check_bit(i as usize) {
    //            map.update(
    //                bg_start + i * cluster_size,
    //                cluster_size,
    //                AllocStatus::Used
    //            );
    //        }
    //    }
    //}

    Ok(())
}


/// Fetches a block group descriptor, based on the number of the block group.
/// Descriptors are read from the first block group. This procedure assumes that the standard
/// layout (not META_BG) is used.
fn fetch_regular_bg_descriptor(bg_num: u64, fs: &Fs) -> anyhow::Result<GroupDescriptor>
{
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    let desc: GroupDescriptor = bincode_opt.deserialize(
        &fs.desc_table[(bg_num * fs.desc_size) as usize..]
    )?;

    Ok(desc)
}


// Source: https://github.com/tytso/e2fsprogs/blob/master/lib/ext2fs/csum.c#L716
/// Verifies the checksum of a group descriptor.
fn verify_desc_csum(desc: &GroupDescriptor, bg_num: u64, fs: &Fs) -> anyhow::Result<bool>
{
    if fs.opts.dyn_cfg.is_none() {
        bail!("cannot verify checksum: dyn_cfg is None");
    }

    let mut desc: GroupDescriptor = *desc;
    let orig_csum = desc.bg_checksum;
    let mut csum: u32;

    if fs.opts.dyn_cfg.unwrap().ro_compat.has_metadata_csum() {
        let bincode_opt = DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();

        desc.bg_checksum = 0;
        let raw_desc = bincode_opt.serialize(&desc)?;
        desc.bg_checksum = orig_csum;

        let bg_num_raw = [
            ((bg_num >> 0)  & 0xff) as u8,
            ((bg_num >> 8)  & 0xff) as u8,
            ((bg_num >> 16) & 0xff) as u8,
            ((bg_num >> 24) & 0xff) as u8,
        ];

        csum = ext4_style_crc32c_le(fs.csum_seed.unwrap(), &bg_num_raw);
        csum = ext4_style_crc32c_le(csum, &raw_desc[..fs.desc_size as usize]);
    } else if fs.opts.dyn_cfg.unwrap().ro_compat.has_gdt_csum() {
        // TODO: support for gdt_csum
        bail!("gdt_csum is not supported");

        #[allow(unreachable_code)]
        if fs.csum_seed.is_none() {
            bail!("cannot verify checksum: checksum seed is not initialised");
        }
    } else {
        bail!("cannot verify checksum: neither of metadata_csum and gdt_csum is set");
    }

    Ok((csum & 0xffff) as u16 == orig_csum)
}


/// Creates FsConfig from a super block and checks it for invalid or unsupported configuration.
fn get_and_check_fs_options(sb: &SuperBlock, cfg: &Config) -> anyhow::Result<FsOptions>
{
    // Constructing enums and flag fields.

    let state = State { 0: sb.s_state };
    let error_policy = match sb.s_errors {
        0 => Some(ErrorPolicy::Null),
        1 => Some(ErrorPolicy::Continue),
        2 => Some(ErrorPolicy::ReadOnly),
        3 => Some(ErrorPolicy::Panic),
        _ => None,
    };
    let fs_creator = match sb.s_creator_os {
        0 => Some(FsCreator::Linux),
        1 => Some(FsCreator::Hurd),
        2 => Some(FsCreator::Masix),
        3 => Some(FsCreator::FreeBSD),
        4 => Some(FsCreator::Linux),
        _ => None,
    };
    let revision = match sb.s_rev_level {
        0 => Some(Revision::GoodOld),
        1 => Some(Revision::Dynamic),
        _ => None,
    };
    let compat = CompatFeatures {
        0: sb.s_feature_compat,
    };
    let incompat = IncompatFeatures {
        0: sb.s_feature_incompat,
    };
    let ro_compat = RoCompatFeatures {
        0: sb.s_feature_ro_compat,
    };
    let def_hash_version = match sb.s_def_hash_version {
        0 => Some(HashVersion::Legacy),
        1 => Some(HashVersion::HalfMD4),
        2 => Some(HashVersion::Tea),
        3 => Some(HashVersion::LegacyUnsigned),
        4 => Some(HashVersion::HalfMD4Unsigned),
        5 => Some(HashVersion::TeaUnsigned),
        6 => Some(HashVersion::SipHash),
        _ => None,
    };
    let def_mount_opts = DefMountOpts {
        0: sb.s_default_mount_opts,
    };
    let flags = Flags { 0: sb.s_flags };
    let mut encrypt_algos: [Option<EncryptAlgo>; 4] = Default::default();
    for (a, b) in encrypt_algos.iter_mut().zip(sb.s_encrypt_algos) {
        *a = match b {
            0 => Some(EncryptAlgo::Null),
            1 => Some(EncryptAlgo::AES256XTS),
            2 => Some(EncryptAlgo::AES256GCM),
            3 => Some(EncryptAlgo::AES256CBC),
            4 => Some(EncryptAlgo::AES256CTS),
            _ => None,
        };
    }

    // Error checking.

    if state.has_unknown() {
        bail!("unknown `s_state` flags: {:#06x}", state.0);
    }
    // NOTE: the presence of the `valid` flag is not checked.
    // NOTE: the presence of the `orphan` flag is ignored.
    if state.has_error() {
        bail!("errors present in the filesystem");
    }
    if state.has_fc_replay() {
        bail!("fast-commit replay is in progress");
    }

    // NOTE: `s_errors` is not that important.
    if error_policy.is_none() {
        bail!("unknown error policy: {:#0x}", sb.s_errors);
    } else if let Some(ErrorPolicy::Null) = error_policy {
        bail!("invalid error policy: {:#0x}", sb.s_errors);
    }

    if fs_creator.is_none() {
        bail!("unknown creator operating system: {:#0x}", sb.s_creator_os);
    }

    if revision.is_none() {
        bail!("unknown revision level: {:#010x}", sb.s_rev_level);
    }

    let mut fs_opts = FsOptions {
        state,
        error_policy: error_policy.unwrap(),
        fs_creator: fs_creator.unwrap(),
        revision: revision.unwrap(),
        dyn_cfg: None,
        journal_cfg: None,
        bit64_cfg: None,
    };

    // --- dynamic revision level only ---

    if let Revision::Dynamic = fs_opts.revision {
        if compat.has_unknown() {
            bail!("unknown `s_feature_compat` flags: {:#010x}", compat.0);
        }
        if compat.has_exclude_inode() {
            bail!("unsupported feature: exclude_inode");
        }
        if compat.has_exclude_bitmap() {
            bail!("unsupported feature: exclude_bitmap");
        }

        if incompat.has_unknown() {
            bail!("unknown `s_feature_incompat` flags: {:#010x}", incompat.0);
        }
        if incompat.has_recover() && !cfg.ignore_recovery {
            bail!("filesystem needs recovery: try to unmount and/or run fsck on the file system");
        }
        if incompat.has_journal_dev() {
            bail!("filesystem has an external journaling device");
        }
        // TODO: Add support for META_BG.
        if incompat.has_meta_bg() {
            bail!("META_BG is not supported due to conflicting documentation");
        }
        if incompat.has_dirdata() {
            bail!("unsupported feature: dirdata");
        }
        if incompat.has_encrypt() {
            bail!("filesystem has encrypted blocks");
        }

        if ro_compat.has_unknown() {
            bail!("unknown `s_feature_ro_compat` flags: {:#010x}", ro_compat.0);
        }
        if ro_compat.has_readonly() && !cfg.ignore_readonly {
            bail!("filesystem is marked as read-only");
        }
        // NOTE: it is unclear what this does.
        // It has to do with allocation and overlapping blocks. It might be viable to perform
        // operations on the system, as long as the allocation of blocks is not altered.
        //
        // Reference: http://lkml.iu.edu/hypermail/linux/kernel/2010.0/04429.html
        if ro_compat.has_shared_blocks() {
            bail!("filesystem has shared blocks");
        }
        if ro_compat.has_metadata_csum() && ro_compat.has_gdt_csum() {
            bail!("gdt_csum and metadata_csum cannot be set at the same time");
        }
        // TODO: Add support for GDT_CSUM.
        if ro_compat.has_gdt_csum() {
            bail!("unsupported feature: gdt_csum");
        }

        fs_opts.dyn_cfg = Some(DynConfig {
            compat,
            incompat,
            ro_compat,
        });
    }

    // --- journalling support only ---

    if compat.has_has_journal() {
        // NOTE: unclear whether siphash should be legal here.
        if def_hash_version.is_none() {
            bail!(
                "unknown default hash version: {:#0x}",
                sb.s_def_hash_version
            )
        }

        // NOTE: the DISCARD mount option could be of relevance here.
        if def_mount_opts.has_unknown() {
            bail!(
                "unknown `s_default_mount_opts` flags: {:#010x}",
                def_mount_opts.0
            );
        }

        fs_opts.journal_cfg = Some(JournalConfig {
            def_hash_ver: def_hash_version.unwrap(),
            def_mount_opts,
        });
    }

    // --- 64-bit support only ---

    if incompat.has_64bit() {
        if flags.has_unknown() {
            bail!("unknown `s_flags` flags: {:#010x}", flags.0);
        }
        if flags.has_fix_snapshot() {
            bail!("snapshot inodes are corrupted");
        }
        if flags.has_fix_exclude() {
            bail!("exclude bitmaps are corrupted");
        }

        fs_opts.bit64_cfg = Some(Bit64Config {
            flags,
            encrypt_algos: None,
        });

        if incompat.has_encrypt() {
            let mut algos: [EncryptAlgo; 4] = Default::default();

            for (i, algo) in encrypt_algos.iter().enumerate() {
                match algo {
                    None => bail!(
                        "unknown encryption algorithm in `s_encrypt_algos`[{}]: {:#0x}",
                        i,
                        sb.s_encrypt_algos[i]
                    ),
                    Some(EncryptAlgo::Null) => {
                        bail!("invalid encryption algorithm in `s_encrypt_algos`[{}]", i)
                    }
                    _ => algos[i] = algo.unwrap(),
                }
            }

            fs_opts.bit64_cfg.as_mut().unwrap().encrypt_algos = Some(algos);
        }
    }

    // --- End of checking ---

    Ok(fs_opts)
}


// Source: https://github.com/FauxFaux/ext4-rs/blob/211fa05cd7b1498060b4b68ffed368d8d3c3b788/src/parse.rs
/// Ext4-style crc32c algorithm.
fn ext4_style_crc32c_le(seed: u32, buf: &[u8]) -> u32
{
    crc::crc32::update(seed ^ (!0), &crc::crc32::CASTAGNOLI_TABLE, buf) ^ (!0u32)
}


/// Calculates the offset of a specified block group.
fn start_of_bg(bg_num: u64, fs: &Fs) -> u64
{
    fs.sb.s_first_data_block as u64 * bs!(fs.sb.s_log_block_size) + bg_num * fs.bg_size
}


/// Returns the offset of the 1st group desriptor table.
fn start_of_first_gdt(sb: &SuperBlock) -> u64
{
    // NOTE: s_first_data_block > 1 is not accounted for.
    if bs!(sb.s_log_block_size) == 1024 {
        2048
    } else {
        bs!(sb.s_log_block_size)
    }
}


// Debug implementations.


impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        let mut flags: Vec<&str> = Vec::new();

        if self.has_valid() {
            flags.push("valid");
        }
        if self.has_error() {
            flags.push("error");
        }
        if self.has_orphan() {
            flags.push("orphan");
        }
        if self.has_fc_replay() {
            flags.push("fc_replay");
        }

        f.debug_struct("State")
            .field("valid", &flags)
            .field("invalid", &self.get_unknown())
            .finish()
    }
}


impl std::fmt::Debug for CompatFeatures {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        let mut flags: Vec<&str> = Vec::new();

        if self.has_dir_prealloc() {
            flags.push("dir_prealloc");
        }
        if self.has_imagic_inodes() {
            flags.push("imagic_inodes");
        }
        if self.has_has_journal() {
            flags.push("has_journal");
        }
        if self.has_ext_attr() {
            flags.push("ext_attr");
        }
        if self.has_resize_inode() {
            flags.push("resize_inode");
        }
        if self.has_dir_index() {
            flags.push("dir_index");
        }
        if self.has_lazy_bg() {
            flags.push("lazy_bg");
        }
        if self.has_exclude_inode() {
            flags.push("exclude_inode");
        }
        if self.has_exclude_bitmap() {
            flags.push("exclude_bitmap");
        }
        if self.has_sparse_super2() {
            flags.push("sparse_super2");
        }
        if self.has_fast_commit() {
            flags.push("fast_commit");
        }
        if self.has_stable_inodes() {
            flags.push("stable_inodes");
        }
        if self.has_orphan_file() {
            flags.push("orphan_file");
        }

        f.debug_struct("CompatFeatures")
            .field("valid", &flags)
            .field("invalid", &self.get_unknown())
            .finish()
    }
}


impl std::fmt::Debug for IncompatFeatures {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        let mut flags: Vec<&str> = Vec::new();

        if self.has_compression() {
            flags.push("compression");
        }
        if self.has_filetype() {
            flags.push("filetype");
        }
        if self.has_recover() {
            flags.push("recover");
        }
        if self.has_journal_dev() {
            flags.push("journal_dev");
        }
        if self.has_meta_bg() {
            flags.push("meta_bg");
        }
        if self.has_extents() {
            flags.push("extents");
        }
        if self.has_64bit() {
            flags.push("64bit");
        }
        if self.has_mmp() {
            flags.push("mmp");
        }
        if self.has_flex_bg() {
            flags.push("flex_bg");
        }
        if self.has_ea_inode() {
            flags.push("ea_inode");
        }
        if self.has_dirdata() {
            flags.push("dirdata");
        }
        if self.has_csum_seed() {
            flags.push("csum_seed");
        }
        if self.has_largedir() {
            flags.push("largedir");
        }
        if self.has_inline_data() {
            flags.push("inline_data");
        }
        if self.has_encrypt() {
            flags.push("encrypt");
        }
        if self.has_casefold() {
            flags.push("casefold");
        }

        f.debug_struct("IncompatFeatures")
            .field("valid", &flags)
            .field("invalid", &self.get_unknown())
            .finish()
    }
}


impl std::fmt::Debug for RoCompatFeatures {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        let mut flags: Vec<&str> = Vec::new();

        if self.has_sparse_super() {
            flags.push("sparse_super");
        }
        if self.has_large_file() {
            flags.push("large_file");
        }
        if self.has_btree_dir() {
            flags.push("btree_dir");
        }
        if self.has_huge_file() {
            flags.push("huge_file");
        }
        if self.has_gdt_csum() {
            flags.push("gdt_csum");
        }
        if self.has_dir_nlink() {
            flags.push("dir_nlink");
        }
        if self.has_extra_isize() {
            flags.push("extra_isize");
        }
        if self.has_has_snapshot() {
            flags.push("has_snapshot");
        }
        if self.has_quota() {
            flags.push("quota");
        }
        if self.has_bigalloc() {
            flags.push("bigalloc");
        }
        if self.has_metadata_csum() {
            flags.push("metadata_csum");
        }
        if self.has_replica() {
            flags.push("replica");
        }
        if self.has_readonly() {
            flags.push("readonly");
        }
        if self.has_project() {
            flags.push("project");
        }
        if self.has_shared_blocks() {
            flags.push("shared_blocks");
        }
        if self.has_verity() {
            flags.push("verity");
        }
        if self.has_orphan_present() {
            flags.push("orphan_present");
        }

        f.debug_struct("RoCompatFeatures")
            .field("valid", &flags)
            .field("invalid", &self.get_unknown())
            .finish()
    }
}


impl std::fmt::Debug for DefMountOpts {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        let mut flags: Vec<&str> = Vec::new();

        if self.has_debug() {
            flags.push("debug");
        }
        if self.has_bsdgroups() {
            flags.push("bsdgroups");
        }
        if self.has_xattr_user() {
            flags.push("xattr_user");
        }
        if self.has_acl() {
            flags.push("acl");
        }
        if self.has_uid16() {
            flags.push("uid16");
        }
        if self.has_jmode_data() {
            flags.push("jmode_data");
        }
        if self.has_jmode_ordered() {
            flags.push("jmode_ordered");
        }
        if self.has_jmode() {
            flags.push("jmode");
        }
        if self.has_jmode_wback() {
            flags.push("jmode_wback");
        }
        if self.has_nobarrier() {
            flags.push("nobarrier");
        }
        if self.has_block_validity() {
            flags.push("block_validity");
        }
        if self.has_discard() {
            flags.push("discard");
        }
        if self.has_nodealloc() {
            flags.push("nodealloc");
        }

        f.debug_struct("DefMountOpts")
            .field("valid", &flags)
            .field("invalid", &self.get_unknown())
            .finish()
    }
}


impl std::fmt::Debug for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        let mut flags: Vec<&str> = Vec::new();

        if self.has_signed_hash() {
            flags.push("signed_hash");
        }
        if self.has_unsigned_hash() {
            flags.push("unsigned_hash");
        }
        if self.has_test() {
            flags.push("test");
        }
        if self.has_is_snapshot() {
            flags.push("is_snapshot");
        }
        if self.has_fix_snapshot() {
            flags.push("fix_snapshot");
        }
        if self.has_fix_exclude() {
            flags.push("fix_exclude");
        }

        f.debug_struct("Flags")
            .field("valid", &flags)
            .field("invalid", &self.get_unknown())
            .finish()
    }
}
