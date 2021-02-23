use anyhow::Result;
use box_format::{BoxFileWriter, BoxPath, Compression};
use std::io::Write;
use std::{
    collections::HashMap,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

const DIVIDER_UUID: u128 = 0xaae8ea9c35484ee4bf28f1a25a6b3c6c;
const DOTNET_INSTALLER: &str = "dotnet5-webinst.exe";
const ONECLICK_INSTALLER: &str = "Divvun.Installer.OneClick.exe";

fn main() -> Result<()> {
    build()?;
    dist()
}

fn build() -> Result<()> {
    Command::new("xargo")
        .current_dir(project_root().join("bundler"))
        .args(&["build", "--release", "--target=i686-pc-windows-msvc"])
        .status()?;

    Ok(())
}

fn dist() -> Result<()> {
    let dist_dir = project_root().join("target").join("dist");
    std::fs::create_dir_all(&dist_dir)?;
    let tmp_dist_file = dist_dir.join("./oneclick-installer.tmp");
    let mut bf = BoxFileWriter::create(&tmp_dist_file)?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .to_le_bytes();

    bf.set_file_attr("created", now.to_vec())?;

    let files = [DOTNET_INSTALLER, ONECLICK_INSTALLER];

    for file in &files {
        let box_path = BoxPath::new(file)?;

        let mut file = std::fs::File::open(&file)?;
        let file_meta = metadata(&file.metadata()?);
        bf.insert(Compression::Zstd, box_path, &mut file, file_meta)?;
    }

    bf.finish()?;

    let dist_file = dist_dir.join(ONECLICK_INSTALLER);

    let mut file = std::fs::OpenOptions::new();
    let file = file.create(true).write(true).open(&dist_file)?;
    let mut writer = BufWriter::new(file);

    let tmp_file = std::fs::File::open(&tmp_dist_file)?;
    let mut reader = BufReader::new(tmp_file);

    let extract_file = std::fs::File::open(
        project_root()
            .join("target")
            .join("i686-pc-windows-msvc")
            .join("release")
            .join("bundler.exe"),
    )?;
    let mut extract_reader = BufReader::new(extract_file);

    std::io::copy(&mut extract_reader, &mut writer)?;
    writer.write_all(&DIVIDER_UUID.to_le_bytes())?;
    std::io::copy(&mut reader, &mut writer)?;

    drop(reader);
    drop(writer);
    std::fs::remove_file(tmp_dist_file)?;

    Ok(())
}

#[inline(always)]
fn metadata(meta: &std::fs::Metadata) -> HashMap<String, Vec<u8>> {
    let mut attrs = HashMap::new();

    macro_rules! attr_systime {
        ($map:ident, $name:expr, $data:expr) => {
            if let Ok(value) = $data {
                let bytes = value
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_le_bytes()
                    .to_vec();

                $map.insert($name.into(), bytes);
            }
        };
    }

    attr_systime!(attrs, "created", meta.created());
    attr_systime!(attrs, "modified", meta.modified());
    attr_systime!(attrs, "accessed", meta.accessed());

    attrs
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
