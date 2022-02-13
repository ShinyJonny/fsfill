#![allow(dead_code)]
use std::io::{Seek, SeekFrom};
use serde::{Serialize, Deserialize};
use bincode::{Options, DefaultOptions};
use anyhow::anyhow;

use crate::{Context, Config};
use crate::serial::Array;

/// The Ext2/3/4 Superblock structure.
/// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct SuperBlock {
    pub s_inodes_count: u32,            /* Inodes count */
    pub s_blocks_count_lo: u32,         /* Blocks count */
    pub s_r_blocks_count_lo: u32,       /* Reserved blocks count */
    pub s_free_blocks_count_lo: u32,    /* Free blocks count */
    pub s_free_inodes_count: u32,       /* Free inodes count */
    pub s_first_data_block: u32,        /* First Data Block */
    pub s_log_block_size: u32,          /* Block size */
    pub s_log_cluster_size: u32,        /* Allocation cluster size */
    pub s_blocks_per_group: u32,        /* # Blocks per group */
    pub s_clusters_per_group: u32,      /* # Clusters per group */
    pub s_inodes_per_group: u32,        /* # Inodes per group */
    pub s_mtime: u32,                   /* Mount time */
    pub s_wtime: u32,                   /* Write time */
    pub s_mnt_count: u16,               /* Mount count */
    pub s_max_mnt_count: u16,           /* Maximal mount count */
    pub s_magic: u16,                   /* Magic signature */
    pub s_state: u16,                   /* File system state */
    pub s_errors: u16,                  /* Behaviour when detecting errors */
    pub s_minor_rev_level: u16,         /* minor revision level */
    pub s_lastcheck: u32,               /* time of last check */
    pub s_checkinterval: u32,           /* max. time between checks */
    pub s_creator_os: u32,              /* OS */
    pub s_rev_level: u32,               /* Revision level */
    pub s_def_resuid: u16,              /* Default uid for reserved blocks */
    pub s_def_resgid: u16,              /* Default gid for reserved blocks */
    /*
     * These fields are for EXT4_DYNAMIC_REV superblocks only.
     *
     * Note: the difference between the compatible feature set and
     * the incompatible feature set is that if there is a bit set
     * in the incompatible feature set that the kernel doesn't
     * know about, it should refuse to mount the filesystem.
     *
     * e2fsck's requirements are more strict; if it doesn't know
     * about a feature in either the compatible or incompatible
     * feature set, it must abort and not try to meddle with
     * things it doesn't understand...
     */
    pub s_first_ino: u32,               /* First non-reserved inode */
    pub s_inode_size: u16,              /* size of inode structure */
    pub s_block_group_nr: u16,          /* block group # of this superblock */
    pub s_feature_compat: u32,          /* compatible feature set */
    pub s_feature_incompat: u32,        /* incompatible feature set */
    pub s_feature_ro_compat: u32,       /* readonly-compatible feature set */
    pub s_uuid: [u8; 16],               /* 128-bit uuid for volume */
    /// Type char[16].
    pub s_volume_name: [u8; 16],        /* volume name */
    /// Type __nonstring char[64].
    pub s_last_mounted: Array<u8, 64>,  /* directory where last mounted */
    pub s_algorithm_usage_bitmap: u32,  /* For compression */
    /*
     * Performance hints.  Directory preallocation should only
     * happen if the EXT4_FEATURE_COMPAT_DIR_PREALLOC flag is on.
     */
    pub s_prealloc_blocks: u8,          /* Nr of blocks to try to preallocate*/
    pub s_prealloc_dir_blocks: u8,      /* Nr to preallocate for dirs */
    /// Named `s_padding1` in Ext2.
    pub s_reserved_gdt_blocks: u16,     /* Per group desc for online growth */
    // --- End of Ext2 superblock ---
    /*
     * Journaling support valid if EXT4_FEATURE_COMPAT_HAS_JOURNAL set.
     */
    pub s_journal_uuid: [u8; 16],       /* uuid of journal superblock */
    pub s_journal_inum: u32,            /* inode number of journal file */
    pub s_journal_dev: u32,             /* device number of journal file */
    pub s_last_orphan: u32,             /* start of list of inodes to delete */
    pub s_hash_seed: [u32; 4],          /* HTREE hash seed */
    pub s_def_hash_version: u8,         /* Default hash version to use */
    /// Named `s_reserved_char_pad` in Ext3.
    pub s_jnl_backup_type: u8,
    /// Named `s_reserved_word_pad` in Ext3.
    pub s_desc_size: u16 ,              /* size of group descriptor */
    pub s_default_mount_opts: u32,
    pub s_first_meta_bg: u32,           /* First metablock block group */
    // --- End of Ext3 superblock ---
    pub s_mkfs_time: u32,               /* When the filesystem was created */
    pub s_jnl_blocks: [u32; 17],        /* Backup of the journal inode */
    /* 64bit support valid if EXT4_FEATURE_COMPAT_64BIT */
    pub s_blocks_count_hi: u32,         /* Blocks count */
    pub s_r_blocks_count_hi: u32,       /* Reserved blocks count */
    pub s_free_blocks_count_hi: u32,    /* Free blocks count */
    pub s_min_extra_isize: u16,         /* All inodes have at least # bytes */
    pub s_want_extra_isize: u16,        /* New inodes should reserve # bytes */
    pub s_flags: u32,                   /* Miscellaneous flags */
    pub s_raid_stride: u16 ,            /* RAID stride */
    pub s_mmp_update_interval: u16 ,    /* # seconds to wait in MMP checking */
    pub s_mmp_block: u64 ,              /* Block for multi-mount protection */
    pub s_raid_stripe_width: u32 ,      /* blocks on all data disks (N*stride)*/
    pub s_log_groups_per_flex: u8,      /* FLEX_BG group size */
    pub s_checksum_type: u8,            /* metadata checksum algorithm used */
    pub s_encryption_level: u8,         /* versioning level for encryption */
    pub s_reserved_pad: u8,             /* Padding to next 32bits */
    pub s_kbytes_written: u64,          /* nr of lifetime kilobytes written */
    pub s_snapshot_inum: u32,           /* Inode number of active snapshot */
    pub s_snapshot_id: u32,             /* sequential ID of active snapshot */
    pub s_snapshot_r_blocks_count: u64, /* reserved blocks for active snapshot's future use */
    pub s_snapshot_list: u32,           /* inode number of the head of the on-disk snapshot list */
    pub s_error_count: u32,             /* number of fs errors */
    pub s_first_error_time: u32,        /* first time an error happened */
    pub s_first_error_ino: u32,         /* inode involved in first error */
    pub s_first_error_block: u64,       /* block involved of first error */
    /// Type __nonstring __u8[32].
    pub s_first_error_func: [u8; 32],   /* function where the error happened */
    pub s_first_error_line: u32,        /* line number where error happened */
    pub s_last_error_time: u32,         /* most recent time of an error */
    pub s_last_error_ino: u32,          /* inode involved in last error */
    pub s_last_error_line: u32,         /* line number where error happened */
    pub s_last_error_block: u64,        /* block involved of last error */
    /// Type __nonstring __u8[32].
    pub s_last_error_func: [u8; 32],    /* function where the error happened */
    pub s_mount_opts: Array<u8, 64>,
    pub s_usr_quota_inum: u32,          /* inode for tracking user quota */
    pub s_grp_quota_inum: u32,          /* inode for tracking group quota */
    pub s_overhead_clusters: u32,       /* overhead blocks/clusters in fs */
    pub s_backup_bgs: [u32; 2],         /* groups with sparse_super2 SBs */
    pub s_encrypt_algos: [u8; 4],       /* Encryption algorithms in use  */
    pub s_encrypt_pw_salt: [u8; 16],    /* Salt used for string2key algorithm */
    pub s_lpf_ino: u32,                 /* Location of the lost+found inode */
    pub s_prj_quota_inum: u32,          /* inode for tracking project quota */
    pub s_checksum_seed: u32,           /* crc32c(uuid) if csum_seed set */
    pub s_wtime_hi: u8,
    pub s_mtime_hi: u8,
    pub s_mkfs_time_hi: u8,
    pub s_lastcheck_hi: u8,
    pub s_first_error_time_hi: u8,
    pub s_last_error_time_hi: u8,
    pub s_first_error_errcode: u8,
    pub s_last_error_errcode: u8   ,
    pub s_encoding: u16 ,               /* Filename charset encoding */
    pub s_encoding_flags: u16 ,         /* Filename charset encoding flags */
    pub s_orphan_file_inum: u32 ,       /* Inode for tracking orphan inodes */
    pub s_reserved: Array<u32, 94>,     /* Padding to the end of the block */
    pub s_checksum: u32,                /* crc32c(superblock) */
}

// sb.s_rev_level
const EXT2_GOOD_OLD_REV: u32 = 0;
const EXT2_DYNAMIC_REV: u32  = 1;

// sb.s_feature_compat
// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h
const EXT4_FEATURE_COMPAT_DIR_PREALLOC: u32      = 0x0001;
const EXT4_FEATURE_COMPAT_IMAGIC_INODES: u32     = 0x0002;
const EXT4_FEATURE_COMPAT_HAS_JOURNAL: u32       = 0x0004;
const EXT4_FEATURE_COMPAT_EXT_ATTR: u32          = 0x0008;
const EXT4_FEATURE_COMPAT_RESIZE_INODE: u32      = 0x0010;
const EXT4_FEATURE_COMPAT_DIR_INDEX: u32         = 0x0020;
const EXT4_FEATURE_COMPAT_SPARSE_SUPER2: u32     = 0x0200;
const EXT4_FEATURE_COMPAT_FAST_COMMIT: u32       = 0x0400;
const EXT4_FEATURE_COMPAT_STABLE_INODES: u32     = 0x0800;
const EXT4_FEATURE_COMPAT_ORPHAN_FILE: u32       = 0x1000;

// sb.s_feature_ro_compat
// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h
const EXT4_FEATURE_RO_COMPAT_SPARSE_SUPER: u32   = 0x0001;
const EXT4_FEATURE_RO_COMPAT_LARGE_FILE: u32     = 0x0002;
const EXT4_FEATURE_RO_COMPAT_BTREE_DIR: u32      = 0x0004;
const EXT4_FEATURE_RO_COMPAT_HUGE_FILE: u32      = 0x0008;
const EXT4_FEATURE_RO_COMPAT_GDT_CSUM: u32       = 0x0010;
const EXT4_FEATURE_RO_COMPAT_DIR_NLINK: u32      = 0x0020;
const EXT4_FEATURE_RO_COMPAT_EXTRA_ISIZE: u32    = 0x0040;
const EXT4_FEATURE_RO_COMPAT_QUOTA: u32          = 0x0100;
const EXT4_FEATURE_RO_COMPAT_BIGALLOC: u32       = 0x0200;
const EXT4_FEATURE_RO_COMPAT_METADATA_CSUM: u32  = 0x0400;
const EXT4_FEATURE_RO_COMPAT_READONLY: u32       = 0x1000;
const EXT4_FEATURE_RO_COMPAT_PROJECT: u32        = 0x2000;
const EXT4_FEATURE_RO_COMPAT_VERITY: u32         = 0x8000;
const EXT4_FEATURE_RO_COMPAT_ORPHAN_PRESENT: u32 = 0x10000;

// sb.s_feature_incompat
// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h
const EXT4_FEATURE_INCOMPAT_COMPRESSION: u32     = 0x0001;
const EXT4_FEATURE_INCOMPAT_FILETYPE: u32        = 0x0002;
const EXT4_FEATURE_INCOMPAT_RECOVER: u32         = 0x0004;
const EXT4_FEATURE_INCOMPAT_JOURNAL_DEV: u32     = 0x0008;
const EXT4_FEATURE_INCOMPAT_META_BG: u32         = 0x0010;
const EXT4_FEATURE_INCOMPAT_EXTENTS: u32         = 0x0040;
const EXT4_FEATURE_INCOMPAT_64BIT: u32           = 0x0080;
const EXT4_FEATURE_INCOMPAT_MMP: u32             = 0x0100;
const EXT4_FEATURE_INCOMPAT_FLEX_BG: u32         = 0x0200;
const EXT4_FEATURE_INCOMPAT_EA_INODE: u32        = 0x0400;
const EXT4_FEATURE_INCOMPAT_DIRDATA: u32         = 0x1000;
const EXT4_FEATURE_INCOMPAT_CSUM_SEED: u32       = 0x2000;
const EXT4_FEATURE_INCOMPAT_LARGEDIR: u32        = 0x4000;
const EXT4_FEATURE_INCOMPAT_INLINE_DATA: u32     = 0x8000;
const EXT4_FEATURE_INCOMPAT_ENCRYPT: u32         = 0x10000;
const EXT4_FEATURE_INCOMPAT_CASEFOLD: u32        = 0x20000;

// TODO
/// Process an Ext2/3/4 file system.
pub fn process_drive(context: &mut Context, _cfg: &Config) -> anyhow::Result<()>
{
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    context.drive.seek(SeekFrom::Start(1024))?;
    let sb: SuperBlock = bincode_opt.deserialize_from(&context.drive)?;

    check_unknown_features(&sb)?;

    println!("{:#?}", sb);

    Ok(())
}

/// Check for unknown features.
/// The program will not touch the filesystem if unknown features are detected.
fn check_unknown_features(sb: &SuperBlock) -> anyhow::Result<()>
{
    // Revision level.

    if sb.s_rev_level > 1 {
        Err(anyhow!("feature check failed: invalid sb.s_rev_level flag: {}", sb.s_rev_level))?
    }
    if sb.s_rev_level == EXT2_GOOD_OLD_REV {
        return Ok(())
    }

    // Compatible features.

    if sb.s_feature_compat & 0x40 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_rev_level flag: 0x40"))?
    }
    if sb.s_feature_compat & 0x80 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_rev_level flag: 0x80"))?
    }
    if sb.s_feature_compat >> 13 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_rev_level flag: above 13 bits"))?
    }

    // Ro-compatible features.

    if sb.s_feature_ro_compat & 0x80 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_feature_ro_compat flag: 0x80"))?
    }
    if sb.s_feature_ro_compat & 0x800 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_feature_ro_compat flag: 0x800"))?
    }
    if sb.s_feature_ro_compat & 0x4000 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_feature_ro_compat flag: 0x4000"))?
    }
    if sb.s_feature_ro_compat >> 17 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_feature_ro_compat flag: above 17 bits"))?
    }

    // Incompatible features.

    if sb.s_feature_incompat & 0x20 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_feature_incompat flag: 0x20"))?
    }
    if sb.s_feature_incompat & 0x800 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_feature_incompat flag: 0x800"))?
    }
    if sb.s_feature_incompat >> 18 != 0 {
        Err(anyhow!("feature check failed: invalid sb.s_feature_incompat flag: above 18 bits"))?
    }

    Ok(())
}
