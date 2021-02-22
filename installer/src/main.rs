#![windows_subsystem = "windows"]

use box_format::BoxFileReader;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

const DIVIDER_UUID: u128 = 0xaae8ea9c35484ee4bf28f1a25a6b3c6c;
fn main() {
    std::process::exit(run());
}

#[inline(always)]
fn run() -> i32 {
    let bf = match open_box_segment() {
        Ok(v) => v,
        Err(e) => return e,
    };

    process(&bf)
}

fn open_box_segment() -> Result<BoxFileReader, i32> {
    let path = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("ERROR: Could not access self-extractor for opening!");
            eprintln!("{:?}", e);
            return Err(1);
        }
    };

    let file = match std::fs::File::open(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR: Could not access self-extractor for opening!");
            eprintln!("{:?}", e);
            return Err(2);
        }
    };

    let mmap = match unsafe { memmap::Mmap::map(&file) } {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR: Could not access self-extractor for opening!");
            eprintln!("{:?}", e);
            return Err(3);
        }
    };

    let boundary = twoway::find_bytes(&mmap[..], &DIVIDER_UUID.to_le_bytes());
    let offset = match boundary {
        Some(v) => v + std::mem::size_of::<u128>(),
        None => {
            eprintln!("ERROR: Could not find embedded .box file data to extract.");
            return Err(4);
        }
    };

    let bf = match box_format::BoxFileReader::open_at_offset(path, offset as u64) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR: Could not read .box data!");
            eprintln!("{:?}", e);
            return Err(5);
        }
    };

    Ok(bf)
}

fn process(bf: &BoxFileReader) -> i32 {
    let tempdir = match tempfile::tempdir() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ERROR: Could not create temporary directory!");
            eprintln!("{:?}", e);
            return 12;
        }
    };

    match bf.extract_all(tempdir.path()) {
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "ERROR: Could not extract files to path '{}'!",
                tempdir.path().display()
            );
            eprintln!("{:?}", e);
            return 11;
        }
    }

    exec(tempdir.path())
}

fn exec(cwd: &std::path::Path) -> i32 {
    let dotnet5_path = cwd.join("dotnet5-webinst.exe");
    let oneclick_path = cwd.join("Divvun.Installer.OneClick.exe");

    match Command::new(&dotnet5_path)
        .args(&["-r", "windowsdesktop", "-v", "5", "-a", "x86"])
        .current_dir(&cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .status()
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("ERROR: Running dotnet installer failed!");
            eprintln!("{:?}", e);
            return 2;
        }
    };

    match Command::new(&oneclick_path)
        .current_dir(&cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("ERROR: Running oneclick installer failed!");
            eprintln!("{:?}", e);
            return 3;
        }
    }

    return 0;
}
