use std::io::{Read, Seek, SeekFrom};
use serde::{Deserialize, Serialize};
use bincode::{DefaultOptions, Options};
use anyhow::bail;

use crate::usage_map::{UsageMap, AllocStatus};
use crate::Context;

use super::inode::{Inode, N_BLOCKS};
use super::Fs;
use crate::bs;
use crate::hilo;

pub const EXTENT_SIZE: usize = 12;

// Reference: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4_extents.h
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ExtentHeader {
    pub eh_magic: u16,
    pub eh_entries: u16,
    pub eh_max: u16,
    pub eh_depth: u16,
    pub eh_generation: u32,
}

pub const EXTENT_HEADER_SIZE: usize = 12;

// Reference: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4_extents.h
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Extent {
    pub ee_block: u32,
    pub ee_len: u16,
    pub ee_start_hi: u16,
    pub ee_start_lo: u32,
}

// Reference: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4_extents.h
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ExtentIdx {
    pub ei_block: u32,
    pub ei_leaf_lo: u32,
    pub ei_leaf_hi: u16,
    pub ei_unused: u16,
}

pub const EXTENT_IDX_SIZE: usize = 12;

// Reference: https://elixir.bootlin.com/linux/latest/source/fs/ext4/ext4_extents.h
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct ExtentTail {
    pub et_checksum: u32,
}

pub const EXTENT_TAIL_SIZE: usize = 4;

/// E2fs extent tree.
#[derive(Clone, Debug)]
pub struct ExtentTree {
    root_node: Node,
}

impl ExtentTree {
    /// Reads the inode's extent tree from the drive.
    pub fn new(inode: &Inode, fs: &Fs, ctx: &mut Context) -> anyhow::Result<Self>
    {
        // Get the elements of inode.i_block.
        let mut i_block = [u8::default(); N_BLOCKS * 4];
        for (ei, element) in inode.i_block.iter().enumerate() {
            for (bi, byte) in element.to_le_bytes().iter().enumerate() {
                i_block[ei * 4 + bi] = *byte;
            }
        }

        // Construct the root node and its subnodes.

        let mut root_node = Node::from_raw(&i_block)?;
        root_node.populate_subnodes(fs, ctx)?;

        Ok(ExtentTree {
            root_node,
        })
    }
}

/// Extent tree node.
#[derive(Clone, Debug)]
struct Node {
    pub header: ExtentHeader,
    pub entries: Entries,
    pub subnodes: Option<Vec<Node>>,
}

impl Node {
    /// Deserialises an extent tree node from raw bytes.
    pub fn from_raw(raw_node: &[u8]) -> anyhow::Result<Self>
    {
        let bincode_opt = DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();

        // Deserialise the extent header.

        let header: ExtentHeader = bincode_opt.deserialize(&raw_node)?;

        // TODO: wrap this error with the node id outside of this procedure.
        if header.eh_magic != 0xf30a {
            bail!("extent tree node's header does not match the magic value");
        }

        // Deserialise the extents or extent indexes.

        let entries = if header.eh_depth == 0 {
            let mut extents = Vec::with_capacity(header.eh_entries as usize);

            for i in 0..header.eh_entries as usize {
                let e_offset = EXTENT_HEADER_SIZE + (i * EXTENT_SIZE);
                let extent: Extent = bincode_opt.deserialize(&raw_node[e_offset..])?;

                extents.push(extent);
            }

            Entries::Extents(extents)
        } else {
            let mut indexes = Vec::with_capacity(header.eh_entries as usize);

            for i in 0..header.eh_entries as usize {
                let e_idx_offset = EXTENT_HEADER_SIZE + (i * EXTENT_IDX_SIZE);
                let e_idx: ExtentIdx = bincode_opt.deserialize(&raw_node[e_idx_offset..])?;

                indexes.push(e_idx);
            }

            Entries::Indexes(indexes)
        };

        Ok(Node {
            header,
            entries,
            subnodes: None,
        })
    }

    /// Populates its subnodes from the disk, recursively.
    pub fn populate_subnodes(&mut self, fs: &Fs, ctx: &mut Context) -> anyhow::Result<()>
    {
        // If the entries are not indexes, we have reached the leaves of the tree.
        let indexes = if let Entries::Indexes(v) = &mut self.entries {
            v
        } else {
            return Ok(())
        };

        self.subnodes = Some(Vec::with_capacity(self.header.eh_entries as usize));
        let mut block_buf = vec![u8::default(); bs!(fs.sb.s_log_block_size) as usize];

        // For each index, read the raw node block from the drive, deserialise it, and populate its
        // subnodes.

        for idx in indexes {
            let block = hilo!(idx.ei_leaf_hi, idx.ei_leaf_lo);

            // Read the raw node block from the drive.
            ctx.drive.seek(SeekFrom::Start(block * bs!(fs.sb.s_log_block_size)))?;
            ctx.drive.read_exact(&mut block_buf)?;

            let mut new_subnode = Self::from_raw(&mut block_buf)?;

            // TODO: test on drives with deeply nested extent trees (not tested yet, only on
            // simple extent trees).

            if new_subnode.header.eh_depth > 0 {
                Self::populate_subnodes(&mut new_subnode, fs, ctx)?;
            }

            self.subnodes.as_mut().unwrap().push(new_subnode);
        }

        Ok(())
    }
}

/// Entries of extent nodes.
#[derive(Clone, Debug)]
enum Entries {
    Extents(Vec<Extent>),
    Indexes(Vec<ExtentIdx>),
}

// TODO: implement extent tree scanning utilising the ExtentTree structure. Currently, this
// procedure implements extent tree parsing on its own.
/// Scans the space occupied by the extent tree.
pub fn scan_extent_tree(
    map: &mut UsageMap,
    inode: &Inode,
    fs: &Fs,
    ctx: &mut Context,
) -> anyhow::Result<()>
{
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    // Get the elements if oninode.i_block.

    let mut i_block = [u8::default(); N_BLOCKS * 4];
    for (ei, element) in inode.i_block.iter().enumerate() {
        for (bi, byte) in element.to_le_bytes().iter().enumerate() {
            i_block[ei * 4 + bi] = *byte;
        }
    }

    // Deserialise the header.

    let e_header: ExtentHeader = bincode_opt.deserialize(&i_block)?;

    if e_header.eh_depth == 0 {
        return Ok(());
    }

    // Deserialise the entries and scan the extent node blocks.

    for i in 0..e_header.eh_entries as usize {
        let e_idx_offset = EXTENT_HEADER_SIZE + (i * EXTENT_IDX_SIZE);
        let e_idx: ExtentIdx = bincode_opt.deserialize(&i_block[e_idx_offset..])?;

        let block = hilo!(e_idx.ei_leaf_hi, e_idx.ei_leaf_lo);
        scan_extent_block(map, block, fs, ctx)?;
    }

    Ok(())
}

/// Scans the space occupied by an extent tree node.
fn scan_extent_block(
    map: &mut UsageMap,
    block: u64,
    fs: &Fs,
    ctx: &mut Context
) -> anyhow::Result<()>
{
    let bincode_opt = DefaultOptions::new()
        .with_fixint_encoding()
        .allow_trailing_bytes();

    // Read the raw node block.

    let mut block_buf = vec![u8::default(); bs!(fs.sb.s_log_block_size) as usize];
    ctx.drive.seek(SeekFrom::Start(block * bs!(fs.sb.s_log_block_size)))?;
    ctx.drive.read_exact(&mut block_buf)?;

    let e_header: ExtentHeader = bincode_opt.deserialize(&block_buf)?;

    // Extent header + entries.
    map.update(
        block * bs!(fs.sb.s_log_block_size),
        EXTENT_HEADER_SIZE as u64 + (e_header.eh_entries as u64 * EXTENT_IDX_SIZE as u64),
        AllocStatus::Used
    );
    // Extent tail
    map.update(
        (block + 1) * bs!(fs.sb.s_log_block_size) - 4,
        4,
        AllocStatus::Used
    );

    if e_header.eh_depth == 0 {
        return Ok(());
    }

    // Recursively walk the tree.
    // NOTE: recursive extent tree scanning is untested.
    // It is hard to get a testing sample that has an extent tree deeper than 1 level.

    for i in 0..e_header.eh_entries as usize {
        let e_idx_offset = EXTENT_HEADER_SIZE + (i * EXTENT_IDX_SIZE);
        let e_idx: ExtentIdx = bincode_opt.deserialize(&block_buf[e_idx_offset..])?;

        let block = hilo!(e_idx.ei_leaf_hi, e_idx.ei_leaf_lo);
        scan_extent_block(map, block, fs, ctx)?;
    }

    Ok(())
}

/// Iterator for ExtentTree.
pub struct ExtentTreeIterator<'t> {
    tree: &'t ExtentTree,
    indices: Vec<usize>,
}

impl<'t> ExtentTreeIterator<'t> {
    pub fn new(tree: &'t ExtentTree) -> Self
    {
        Self {
            tree,
            indices: vec![0; tree.root_node.header.eh_depth as usize + 1],
        }
    }

    /// Performs a single walk down the tree, following the path laid out by the indices.
    /// After each walk, the last element in indices (the path) is incremented. When the path
    /// becomes invalid (an index exceeds the number of nodes), the invalid index is set to 0 and
    /// the previous index is incremented. Thus, by repeatedly executing this method, all of the
    /// possible paths are eventually taken.
    /// When a valid path is taken, the method returns the leaf (extent). Thus, all of the tree's
    /// leaves are eventually iterated.
    fn try_find_element(&mut self) -> SearchResult<<Self as Iterator>::Item>
    {
        if self.indices[0] >= self.tree.root_node.header.eh_entries as usize {
            return SearchResult::End;
        }

        let mut cur_node = &self.tree.root_node;
        let mut cur_node_i = 0;

        // Walk the tree to the appropriate node.

        while cur_node.header.eh_depth > 0 {
            if cur_node_i >= self.indices.len() {
                panic!("extent tree branches are longer than root node's eh_depth");
            }

            let cur_subnodes = cur_node.subnodes.as_ref().unwrap();

            if self.indices[cur_node_i] >= cur_subnodes.len() {
                self.indices[cur_node_i - 1] += 1;

                // Zero all the indices down the path.
                for i in &mut self.indices[cur_node_i..] {
                    *i = 0;
                }

                return SearchResult::BadPath;
            }

            cur_node = &cur_subnodes[self.indices[cur_node_i]];
            cur_node_i += 1;
        }

        // TODO: remove this check.
        assert!(cur_node.subnodes.is_none());

        let extents = if let Entries::Extents(v) = &cur_node.entries {
            v
        } else {
            panic!("extent tree: leaf node has indexes instead of extents");
        };

        if self.indices[cur_node_i] >= extents.len() {
            self.indices[cur_node_i - 1] += 1;
            self.indices[cur_node_i] = 0;

            return SearchResult::BadPath;
        }

        let result = &extents[self.indices[cur_node_i]];
        self.indices[cur_node_i] += 1;

        SearchResult::Found(result)
    }
}

impl<'t> Iterator for ExtentTreeIterator<'t> {
    type Item = &'t Extent;

    fn next(&mut self) -> Option<Self::Item>
    {
        // Walk the tree repeatedly until a leaf is found.
        // If the last possible path in the tree is reached, the iterator has been exhausted.
        loop {
            match self.try_find_element() {
                SearchResult::BadPath => (),
                SearchResult::Found(item) => break Some(item),
                SearchResult::End => break None,
            }
        }
    }
}

/// Result of a search in one path down the tree.
enum SearchResult<T> {
    /// Value has been found.
    Found(T),
    /// The path down the tree is invalid.
    BadPath,
    /// The search space has been exhausted.
    End,
}

// Tests

// TODO: test ExtentTree.
