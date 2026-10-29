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
            ├── <date_prefix>_<callsign>.igc
            ├── results.csv
            ├── results.html
            ├── task.cup
            ├── task.html
            └── task.json
```

`results.html` and `task.html` are the unmodified SoaringSpot pages of that day. `task.json` is the task data embedded in the task page, and `task.cup` is the same task in the [SeeYou CUP format](https://github.com/naviter/seeyou_file_formats/blob/main/CUP_file_format.md). These files are replaced on every run, unless the new data can't be read, in which case the existing files are kept and the tool exits with an error after downloading everything else. See [docs/soaringspot-data.md](docs/soaringspot-data.md) for details on the data.

`results.csv` contains one row per contestant with the columns `rank`, `cn`, `contestant`, `countries`, `club`, `glider`, `handicap`, `start_time_local`, `finish_time_local`, `duration`, `speed_kmh`, `distance_km`, `points`, `points_sort_key`, `penalty_points`, and `remarks`. `countries` contains the space-separated country codes of the contestant, for example `za de` for an international two-seater crew. Times are in the local time of the competition, not of your computer. `points_sort_key` is the value that SoaringSpot sorts the points by. While SoaringSpot has no results for the day yet, `results.csv` isn't written, and previously saved `results.html` and `results.csv` files are kept.

For example:

```
./
└── 39th-fai-world-gliding-championships-tabor-2025/
    ├── club/
    │   ├── 2025-06-12/
    │   │   ├── 56C_ABC.igc
    │   │   ├── 56C_XYZ.igc
    │   │   ├── ...
    │   │   ├── results.csv
    │   │   ├── results.html
    │   │   ├── task.cup
    │   │   ├── task.html
    │   │   └── task.json
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
