use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use clap::Parser;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::Digest;
use std::{
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// A simple file comparison tool.
///
/// Files can either be compared by data directly (default), SHA256 or quickly by size and modification date.
#[derive(Parser, Debug)]
struct Cli {
    path1: PathBuf,
    path2: PathBuf,
    /// Compare with SHA256 hashes
    #[arg(short = 's', long)]
    hash: bool,
    /// Compare only using file size and last modification date.
    #[arg(short, long)]
    quick: bool,
    /// Print file comparison data, e.g. hash or size and date.
    #[arg(long, short)]
    print: bool,
}

fn hash_file(path: &Path, file_number: u32) -> Result<[u8; 32]> {
    let mut file =
        fs::File::open(path).with_context(|| format!("Failed to read file at path:\n{path:?}"))?;

    let mut hasher = sha2::Sha256::new();

    let mut buffer = [0; 8192];

    println!("\nHashing file {file_number}");

    let bar = ProgressBar::new(file.metadata()?.len());
    let style = ProgressStyle::default_bar()
        .template("{elapsed_precise} {wide_bar} {bytes} / {total_bytes}")
        .unwrap();
    bar.set_style(style);

    loop {
        let count = file
            .read(&mut buffer)
            .with_context(|| format!("Failed to read file at path:\n{path:?}"))?;

        if count == 0 {
            break;
        }

        sha2::digest::Update::update(&mut hasher, &buffer[..count]);
        bar.inc(count as u64);
    }

    bar.finish_and_clear();

    println!("Finished hashing file {file_number}");

    Ok(hasher.finalize().into())
}

fn get_filenames(path1: &Path, path2: &Path) -> (OsString, OsString) {
    let filename1 = path1.file_name().unwrap_or(path1.as_os_str());
    let filename2 = path2.file_name().unwrap_or(path2.as_os_str());

    (filename1.to_owned(), filename2.to_owned())
}

fn compare_hashes(path1: &Path, path2: &Path, print: bool) -> Result<()> {
    if !size_precheck(path1, path2, print)? {
        return Ok(());
    }

    let hash1 = hash_file(path1, 1)?;
    let hash2 = hash_file(path2, 2)?;

    let (filename1, filename2) = get_filenames(path1, path2);

    if hash1 == hash2 {
        println!(
            "\nCompared files with SHA256, {filename1:?} and {filename2:?}\n\nThey are {}",
            style("identical").bold().green()
        )
    } else {
        println!(
            "\nCompared files with SHA256, {filename1:?} and {filename2:?}\n\nThey are {}",
            style("different").bold().red()
        )
    };

    if print {
        println!("\nHashes:\n{}\n{}", hex::encode(hash1), hex::encode(hash2))
    }

    Ok(())
}

fn size_precheck(path1: &Path, path2: &Path, print: bool) -> Result<bool> {
    let size1 = fs::metadata(path1)
        .with_context(|| format!("Failed to read file at path:\n{path1:?}"))?
        .len();
    let size2 = fs::metadata(path2)
        .with_context(|| format!("Failed to read file at path:\n{path2:?}"))?
        .len();

    if size1 != size2 {
        println!("The files have different sizes and cannot be identical");

        if print {
            println!(
                "File 1: {}\nFile2: {}",
                format_file_size(size1),
                format_file_size(size2)
            );
        }

        return Ok(false);
    }
    Ok(true)
}

fn format_file_size(size: u64) -> String {
    if size >= 1 << 40 {
        format!(
            "{:.2} TB ({} bytes)",
            size as f64 / (1u64 << 40) as f64,
            size
        )
    } else if size >= 1 << 30 {
        format!(
            "{:.2} GB ({} bytes)",
            size as f64 / (1u64 << 30) as f64,
            size
        )
    } else if size >= 1 << 20 {
        format!(
            "{:.2} MB ({} bytes)",
            size as f64 / (1u64 << 20) as f64,
            size
        )
    } else if size >= 1 << 10 {
        format!(
            "{:.2} KB ({} bytes)",
            size as f64 / (1u64 << 10) as f64,
            size
        )
    } else {
        format!("{} bytes", size)
    }
}

fn quick_compare(path1: &Path, path2: &Path, print: bool) -> Result<()> {
    let metadata1 =
        fs::metadata(path1).with_context(|| format!("Failed to read file at path:\n{path1:?}"))?;
    let size1 = metadata1.len();
    let modified1 = metadata1.modified().with_context(|| {
        format!("Failed to read modification date for file at path:\n{path1:?}")
    })?;

    let metadata2 =
        fs::metadata(path2).with_context(|| format!("Failed to read file at path:\n{path2:?}"))?;
    let size2 = metadata2.len();
    let modified2 = metadata2.modified().with_context(|| {
        format!("Failed to read modification date for file at path:\n{path2:?}")
    })?;

    let (filename1, filename2) = get_filenames(path1, path2);

    if size1 == size2 && modified1 == modified2 {
        println!(
            "\nCompared files by size and modification date, {filename1:?} and {filename2:?}\n\nThey seem {}",
            style("identical").bold().green()
        )
    } else if size1 == size2 {
        println!(
            "\nCompared files by size and modification date, {filename1:?} and {filename2:?}\n\nThey are the same size but the last modification dates are {}. Contents were not checked",
            style("different").bold().red()
        )
    } else {
        println!(
            "\nCompared files by size and modification date, {filename1:?} and {filename2:?}\n\nThey are {}. Contents were not checked",
            style("different").bold().red()
        )
    };

    if print {
        let date1: DateTime<Local> = modified1.into();
        let date2: DateTime<Local> = modified2.into();

        println!(
            "\nFile 1\nSize: {}\nModification date: {}",
            format_file_size(size1),
            date1.format("%Y:%m:%d %H:%M:%S")
        );
        println!(
            "\nFile 2\nSize: {}\nModification date: {}",
            format_file_size(size2),
            date2.format("%Y:%m:%d %H:%M:%S")
        );
    }

    Ok(())
}

fn compare_bytes(path1: &Path, path2: &Path, print: bool) -> Result<()> {
    const CHUNK_SIZE: usize = 64 * 1024; // 64 kB

    if !size_precheck(path1, path2, print)? {
        return Ok(());
    }

    let mut file1 = fs::File::open(path1)
        .with_context(|| format!("Failed to read file at path:\n{path1:?}"))?;
    let mut file2 = fs::File::open(path2)
        .with_context(|| format!("Failed to read file at path:\n{path2:?}"))?;

    let mut buffer1 = [0u8; CHUNK_SIZE];
    let mut buffer2 = [0u8; CHUNK_SIZE];

    let (filename1, filename2) = get_filenames(path1, path2);

    let bar = ProgressBar::new(file1.metadata()?.len());
    let style = ProgressStyle::default_bar()
        .template("{elapsed_precise} {wide_bar} {bytes} / {total_bytes}")
        .unwrap();
    bar.set_style(style);

    loop {
        let count1 = file1
            .read(&mut buffer1)
            .with_context(|| format!("Failed to read file at path:\n{path1:?}"))?;
        let count2 = file2
            .read(&mut buffer2)
            .with_context(|| format!("Failed to read file at path:\n{path2:?}"))?;

        if count1 == 0 && count2 == 0 {
            bar.finish_and_clear();
            println!(
                "\nCompared files by contents, {filename1:?} and {filename2:?}\n\nThe data is {}",
                console::style("identical").bold().green()
            );
            break;
        } else if count1 != count2 || buffer1[..count1] != buffer2[..count2] {
            bar.finish_and_clear();
            println!(
                "\nCompared files by contents, {filename1:?} and {filename2:?}\n\nThe data is {}",
                console::style("different").bold().red()
            );
            break;
        }

        bar.inc(count1 as u64);
    }

    if print {
        let metadata1 = fs::metadata(path1)
            .with_context(|| format!("Failed to read file at path:\n{path1:?}"))?;
        let size1 = metadata1.len();

        let metadata2 = fs::metadata(path2)
            .with_context(|| format!("Failed to read file at path:\n{path2:?}"))?;
        let size2 = metadata2.len();

        println!(
            "File 1: {}\nFile 2: {}",
            format_file_size(size1),
            format_file_size(size2)
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.quick {
        quick_compare(&cli.path1, &cli.path2, cli.print)?;
    } else if cli.hash {
        compare_hashes(&cli.path1, &cli.path2, cli.print)?;
    } else {
        compare_bytes(&cli.path1, &cli.path2, cli.print)?;
    }

    Ok(())
}
