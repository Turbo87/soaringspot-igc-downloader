# SoaringSpot IGC Downloader

A command-line tool for downloading IGC flight log files from [SoaringSpot](https://www.soaringspot.com/) gliding competition results.

## Description

SoaringSpot IGC Downloader is a Rust application that allows you to easily download IGC flight log files from gliding competitions hosted on SoaringSpot. It can download files for an entire competition, a specific glider class, or a specific day's task.

The tool automatically organizes downloaded files into a directory structure based on competition name, glider class, and date, and names the files according to IGC naming conventions.

## Installation

### Prerequisites

- Rust and Cargo (install from [rustup.rs](https://rustup.rs/))

### Building from source

```bash
# Clone the repository
git clone https://github.com/yourusername/soaringspot-igc-downloader.git
cd soaringspot-igc-downloader

# Build the project
cargo build --release

# The binary will be available at target/release/soaringspot-igc-downloader[.exe]
```

## Usage

```bash
# Basic usage
soaringspot-igc-downloader <URL> [OPTIONS]

# Download all IGC files from a competition
soaringspot-igc-downloader https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025

# Download all IGC files for a specific class
soaringspot-igc-downloader https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025/results/standard

# Download IGC files for a specific day's task
soaringspot-igc-downloader https://www.soaringspot.com/en_gb/39th-fai-world-gliding-championships-tabor-2025/results/club/task-4-on-2025-06-12/daily

# Specify an output directory
soaringspot-igc-downloader <URL> --output /path/to/output/directory
```

## File Organization

The downloaded files are organized in the following directory structure:

```
<output_directory>/
└── <competition_name>/
    └── <class_name>/
        └── <date>/
            └── <date_prefix>_<callsign>.igc
```

For example:

```
./
└── 39th-fai-world-gliding-championships-tabor-2025/
    ├── club/
    │   ├── 2025-06-12/
    │   │   ├── 56C_ABC.igc
    │   │   ├── 56C_XYZ.igc
    │   │   └── ...
    │   └── 2025-06-13/
    │       └── ...
    ├── standard/
    │   └── ...
    └── 15-meter/
        └── ...
```

## License

This project is open source and licensed under either of these:

- Apache License, Version 2.0, ([LICENSE-APACHE](./LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](./LICENSE-MIT) or https://opensource.org/licenses/MIT)
