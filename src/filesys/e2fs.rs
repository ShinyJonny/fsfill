use std::io::{Seek, SeekFrom};
use anyhow::anyhow;
use serde::{Serialize, Deserialize};
use bincode::{Options, DefaultOptions};

use crate::{Context, Config};
use crate::serial::Array;

/// The Ext2/3/4 Superblock structure.
/// Source: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4.h
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
struct SuperBlock {
    s_inodes_count: u32,            /* Inodes count */
    s_blocks_count_lo: u32,         /* Blocks count */
    s_r_blocks_count_lo: u32,       /* Reserved blocks count */
    s_free_blocks_count_lo: u32,    /* Free blocks count */
    s_free_inodes_count: u32,       /* Free inodes count */
    s_first_data_block: u32,        /* First Data Block */
    s_log_block_size: u32,          /* Block size */
    s_log_cluster_size: u32,        /* Allocation cluster size */
    s_blocks_per_group: u32,        /* # Blocks per group */
    s_clusters_per_group: u32,      /* # Clusters per group */
    s_inodes_per_group: u32,        /* # Inodes per group */
    s_mtime: u32,                   /* Mount time */
    s_wtime: u32,                   /* Write time */
    s_mnt_count: u16,               /* Mount count */
    s_max_mnt_count: u16,           /* Maximal mount count */
    s_magic: u16,                   /* Magic signature */
    s_state: u16,                   /* File system state */
    s_errors: u16,                  /* Behaviour when detecting errors */
    s_minor_rev_level: u16,         /* minor revision level */
    s_lastcheck: u32,               /* time of last check */
    s_checkinterval: u32,           /* max. time between checks */
    s_creator_os: u32,              /* OS */
    s_rev_level: u32,               /* Revision level */
    s_def_resuid: u16,              /* Default uid for reserved blocks */
    s_def_resgid: u16,              /* Default gid for reserved blocks */
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
    s_first_ino: u32,               /* First non-reserved inode */
    s_inode_size: u16,              /* size of inode structure */
    s_block_group_nr: u16,          /* block group # of this superblock */
    s_feature_compat: u32,          /* compatible feature set */
    s_feature_incompat: u32,        /* incompatible feature set */
    s_feature_ro_compat: u32,       /* readonly-compatible feature set */
    s_uuid: [u8; 16],               /* 128-bit uuid for volume */
    /// Type char[16].
    s_volume_name: [u8; 16],        /* volume name */
    /// Type __nonstring char[64].
    s_last_mounted: Array<u8, 64>,  /* directory where last mounted */
    s_algorithm_usage_bitmap: u32,  /* For compression */
    /*
     * Performance hints.  Directory preallocation should only
     * happen if the EXT4_FEATURE_COMPAT_DIR_PREALLOC flag is on.
     */
    s_prealloc_blocks: u8,          /* Nr of blocks to try to preallocate*/
    s_prealloc_dir_blocks: u8,      /* Nr to preallocate for dirs */
    /// Named `s_padding1` in Ext2.
    s_reserved_gdt_blocks: u16,     /* Per group desc for online growth */
    // --- End of Ext2 superblock ---
    /*
     * Journaling support valid if EXT4_FEATURE_COMPAT_HAS_JOURNAL set.
     */
    s_journal_uuid: [u8; 16],       /* uuid of journal superblock */
    s_journal_inum: u32,            /* inode number of journal file */
    s_journal_dev: u32,             /* device number of journal file */
    s_last_orphan: u32,             /* start of list of inodes to delete */
    s_hash_seed: [u32; 4],          /* HTREE hash seed */
    s_def_hash_version: u8,         /* Default hash version to use */
    /// Named `s_reserved_char_pad` in Ext3.
    s_jnl_backup_type: u8,
    /// Named `s_reserved_word_pad` in Ext3.
    s_desc_size: u16 ,              /* size of group descriptor */
    s_default_mount_opts: u32,
    s_first_meta_bg: u32,           /* First metablock block group */
    // --- End of Ext3 superblock ---
    s_mkfs_time: u32,               /* When the filesystem was created */
    s_jnl_blocks: [u32; 17],        /* Backup of the journal inode */
    /* 64bit support valid if EXT4_FEATURE_COMPAT_64BIT */
    s_blocks_count_hi: u32,         /* Blocks count */
    s_r_blocks_count_hi: u32,       /* Reserved blocks count */
    s_free_blocks_count_hi: u32,    /* Free blocks count */
    s_min_extra_isize: u16,         /* All inodes have at least # bytes */
    s_want_extra_isize: u16,        /* New inodes should reserve # bytes */
    s_flags: u32,                   /* Miscellaneous flags */
    s_raid_stride: u16 ,            /* RAID stride */
    s_mmp_update_interval: u16 ,    /* # seconds to wait in MMP checking */
    s_mmp_block: u64 ,              /* Block for multi-mount protection */
    s_raid_stripe_width: u32 ,      /* blocks on all data disks (N*stride)*/
    s_log_groups_per_flex: u8,      /* FLEX_BG group size */
    s_checksum_type: u8,            /* metadata checksum algorithm used */
    s_encryption_level: u8,         /* versioning level for encryption */
    s_reserved_pad: u8,             /* Padding to next 32bits */
    s_kbytes_written: u64,          /* nr of lifetime kilobytes written */
    s_snapshot_inum: u32,           /* Inode number of active snapshot */
    s_snapshot_id: u32,             /* sequential ID of active snapshot */
    s_snapshot_r_blocks_count: u64, /* reserved blocks for active snapshot's future use */
    s_snapshot_list: u32,           /* inode number of the head of the on-disk snapshot list */
    s_error_count: u32,             /* number of fs errors */
    s_first_error_time: u32,        /* first time an error happened */
    s_first_error_ino: u32,         /* inode involved in first error */
    s_first_error_block: u64,       /* block involved of first error */
    /// Type __nonstring __u8[32].
    s_first_error_func: [u8; 32],   /* function where the error happened */
    s_first_error_line: u32,        /* line number where error happened */
    s_last_error_time: u32,         /* most recent time of an error */
    s_last_error_ino: u32,          /* inode involved in last error */
    s_last_error_line: u32,         /* line number where error happened */
    s_last_error_block: u64,        /* block involved of last error */
    /// Type __nonstring __u8[32].
    s_last_error_func: [u8; 32],    /* function where the error happened */
    s_mount_opts: Array<u8, 64>,
    s_usr_quota_inum: u32,          /* inode for tracking user quota */
    s_grp_quota_inum: u32,          /* inode for tracking group quota */
    s_overhead_clusters: u32,       /* overhead blocks/clusters in fs */
    s_backup_bgs: [u32; 2],         /* groups with sparse_super2 SBs */
    s_encrypt_algos: [u8; 4],       /* Encryption algorithms in use  */
    s_encrypt_pw_salt: [u8; 16],    /* Salt used for string2key algorithm */
    s_lpf_ino: u32,                 /* Location of the lost+found inode */
    s_prj_quota_inum: u32,          /* inode for tracking project quota */
    s_checksum_seed: u32,           /* crc32c(uuid) if csum_seed set */
    s_wtime_hi: u8,
    s_mtime_hi: u8,
    s_mkfs_time_hi: u8,
    s_lastcheck_hi: u8,
    s_first_error_time_hi: u8,
    s_last_error_time_hi: u8,
    s_first_error_errcode: u8,
    s_last_error_errcode: u8   ,
    s_encoding: u16 ,               /* Filename charset encoding */
    s_encoding_flags: u16 ,         /* Filename charset encoding flags */
    s_orphan_file_inum: u32 ,       /* Inode for tracking orphan inodes */
    s_reserved: Array<u32, 94>,     /* Padding to the end of the block */
    s_checksum: u32,                /* crc32c(superblock) */
}

// TODO
/// Process an Ext2 file system.
pub fn process_ext2(_context: &mut Context, _cfg: &Config) -> anyhow::Result<()>
{
    Err(anyhow!("dummy"))
}

// TODO
/// Process an Ext3 file system.
pub fn process_ext3(_context: &mut Context, _cfg: &Config) -> anyhow::Result<()>
{
    Err(anyhow!("dummy"))
}

// TODO
/// Process an Ext4 file system.
pub fn process_ext4(context: &mut Context, _cfg: &Config) -> anyhow::Result<()>
{
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    context.drive.seek(SeekFrom::Start(1024))?;
    let sb: SuperBlock = bincode_opt.deserialize_from(&context.drive)?;

    println!("{:#?}", sb);

    Ok(())
}
