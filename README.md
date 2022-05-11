# Zbfill

Zero-fill unused space in file systems

## Description

Scans the file system for unused space and fills it with bytes.
Can be used for file systems within encryption layers like (dm-crypt)[https://wiki.archlinux.org/title/Dm-crypt] to encrypt all the blocks that were not touched by the file system, restoring plausible deniability.

Currently supported file systems:
- Ext2
- Ext3
- Ext4

## Usage

To scan and fill a drive, run:
```
zbfill <DRIVE_PATH>
```

To specify the fill mode, use the `-f` or `--fill-mode` flags:
```
zbfill --fill-mode chacha20 <DRIVE_PATH>
```

To get the usage of the drive in JSON format, use either the `-r` or `--report-only` flags:
```
zbfill --report-only <DRIVE_PATH>
```

For more verbose log output use either the `-v` or `--verbose` flags (can be used multiple times for increased depth of verbosity):
```
zbfill -vv <DRIVE_PATH>
```

To store the informational output into a file, supply a log file with either the `-l` or `--log-file` flags:
```
zbfill -l <LOG_FILE_PATH> <DRIVE_PATH>
```

For more information on the usage and supported flags, use either the `-h` or `--help` flags:
```
zbfill --help
```

## Building

Requirements:
- cargo

Steps:
1. Run: `cargo build --release`

The binary will be located at `target/release/zbfill`

## Installation

Requirements:
- cargo

Steps:
1. Navigate to the directory of the repository.
2. Run: `cargo install --path .`

If you have a correctly setup rust tool chain, the built binary should be in your PATH.

## Third Party Libraries

- [clap](https://crates.io/crates/clap)
- [serde](https://crates.io/crates/serde)
- [bincode](https://crates.io/crates/bincode)
- [serde_json](https://crates.io/crates/serde_json)
- [anyhow](https://crates.io/crates/anyhow)
- [crc](https://crates.io/crates/crc)
- [rand](https://crates.io/crates/rand)
- [rand_chacha](https://crates.io/crates/rand_chacha)
- [rand_hc](https://crates.io/crates/rand_hc)
