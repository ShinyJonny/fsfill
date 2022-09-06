# Fsfill

(WIP) Zero-fill unused space in file systems

## Description

Scans the file system for unused space and fills it with bytes.
Can be used for file systems within encryption layers like [dm-crypt](https://wiki.archlinux.org/title/Dm-crypt) to encrypt all the blocks that were not touched by the file system and restore plausible deniability.

Currently supported file systems:
- Ext4
- Ext2 (experimental)
- Ext3 (experimental)

## Usage

To scan and fill a drive, run:
```
fsfill <DRIVE_PATH>
```

To specify the fill mode, use the `-f` or `--fill-mode` flags:
```
fsfill --fill-mode chacha20 <DRIVE_PATH>
```

To get the usage of the drive in JSON format, use either the `-r` or `--report-only` flags:
```
fsfill --report-only <DRIVE_PATH>
```

For more verbose log output use either the `-v` or `--verbose` flags (can be used multiple times for increased depth of verbosity):
```
fsfill -vv <DRIVE_PATH>
```

To store the informational output into a file, supply a log file with either the `-l` or `--log-file` flags:
```
fsfill -l <LOG_FILE_PATH> <DRIVE_PATH>
```

For more information on the usage and supported flags, use either the `-h` or `--help` flags:
```
fsfill --help
```

## Building

Requirements:
- cargo

Steps:
1. Run: `cargo build --release`

The binary will be located at `target/release/fsfill`

## Installation

Requirements:
- cargo

Steps:
1. Navigate to the directory of the repository.
2. Run: `cargo install --path .`

If you have a correctly setup rust tool chain, the built binary should be in your PATH.

## Known Bugs

- E2fs versions other than Ext4 (Ext2 and Ext3) are not very well supported yet and often get corrupted (WIP).
- Sometimes, a few small 'holes', i.e. unused and unfilled spaces, can still remain on the drive.
