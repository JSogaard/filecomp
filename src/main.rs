use std::{
    ffi::OsString, fs, io::Read, path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use clap::Parser;
use sha2::Digest;

/// A simple file comparison tool.
/// 
/// Files can either be compared with SHA256 (default) or quickly by size and modification date.
#[derive(Parser, Debug)]
struct Cli {
    path1: PathBuf,
    path2: PathBuf,
    /// Compare only using file size and last modification date.
    #[arg(short, long, )]
    quick: bool,
    /// Print file comparison data, e.g. hash or size and date.
    #[arg(long, short)]
    print: bool,
}

fn hash_file(path: &Path) -> Result<[u8; 32]> {
    let mut file =
        fs::File::open(path).with_context(|| format!("Failed to read file at path:\n{path:?}"))?;

    let mut hasher = sha2::Sha256::new();

    let mut buffer = [0; 8192];

    loop {
        let count = file
            .read(&mut buffer)
            .with_context(|| format!("Failed to read file at path:\n{path:?}"))?;

        if count == 0 {
            break;
        }

        sha2::digest::Update::update(&mut hasher, &buffer[..count]);
    }

    Ok(hasher.finalize().into())
}

fn get_filenames(path1: &Path, path2: &Path) -> (OsString, OsString) {
    let filename1 = path1.file_name().unwrap_or(path1.as_os_str());
    let filename2 = path2.file_name().unwrap_or(path2.as_os_str());

    (filename1.to_owned(), filename2.to_owned())
}

fn compare_hashes(path1: &Path, path2: &Path, print: bool) -> Result<()> {
    let hash1 = hash_file(path1)?;
    let hash2 = hash_file(path2)?;

   let (filename1, filename2) = get_filenames(path1, path2);

    if hash1 == hash2 {
        println!("Compared files with SHA256, {filename1:?} and {filename2:?}\n\nThey are identical")
    } else {
        println!("Compared files with SHA256, {filename1:?} and {filename2:?}\n\nThey are different")
    };

    if print {
        println!("\nHashes:\n{}\n{}", hex::encode(hash1), hex::encode(hash2))
    }

    Ok(())
}

fn format_file_size(size: u64) -> String {
    let output;
    if size >= 1 << 40 {
        output  = format!("{:.2} TB ({} bytes)", size as f64 / (1u64 << 40) as f64, size)
    } else if size >= 1 << 30 {
        output  = format!("{:.2} GB ({} bytes)", size as f64 / (1u64 << 30) as f64, size)
    } else if size >= 1 << 20 {
        output  = format!("{:.2} MB ({} bytes)", size as f64 / (1u64 << 20) as f64, size)
    } else if size >= 1 << 10 {
        output  = format!("{:.2} KB ({} bytes)", size as f64 / (1u64 << 10) as f64, size)
    } else {
        output  = format!("{} bytes", size)
    }
    output
}

fn quick_compare(path1: &Path, path2: &Path, print: bool) -> Result<()> {
    let metadata1 = fs::metadata(path1).with_context(|| format!("Failed to read file at path:\n{path1:?}"))?;
    let size1 = metadata1.len();
    let modified1 = metadata1.modified().with_context(|| format!("Failed to read modification date for file at path:\n{path1:?}"))?;

    let metadata2 = fs::metadata(path2).with_context(|| format!("Failed to read file at path:\n{path2:?}"))?;
    let size2 = metadata2.len();
    let modified2 = metadata2.modified().with_context(|| format!("Failed to read modification date for file at path:\n{path2:?}"))?;

    let (filename1, filename2) = get_filenames(path1, path2);

    if size1 == size2 && modified1 == modified2 {
        println!("Compared files by size and modification date, {filename1:?} and {filename2:?}\n\nThey seem identical")
    } else {
        println!("Compared files by size and modification date, {filename1:?} and {filename2:?}\n\nThey are different")
    };

    if print {
        let date1: DateTime<Local> = modified1.into();
        let date2: DateTime<Local> = modified2.into();

        println!("\nFile 1\nSize: {}\nModification date: {}", format_file_size(size1), date1.format("%Y:%m:%d %H:%M:%S"));
        println!("\nFile 2\nSize: {}\nModification date: {}", format_file_size(size2), date2.format("%Y:%m:%d %H:%M:%S"));
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.quick {
        quick_compare(&cli.path1, &cli.path2, cli.print)?;
    } else {
        compare_hashes(&cli.path1, &cli.path2, cli.print)?;
    }

    Ok(())
}
