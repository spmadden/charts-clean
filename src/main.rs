use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::fs::DirEntry;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use irox_log::log::{debug, error, info};
use irox_time::format::{FormatError, FormatParser};
use irox_time::format::iso8601::BASIC_CALENDAR_DATE;
use irox_time::gregorian::Date;

#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
    FormatError(irox_time::format::FormatError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IOError(e) => write!(f, "IOError: {e}"),
            Error::FormatError(e) => write!(f, "FormatError: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IOError(value)
    }
}

impl From<FormatError> for Error {
    fn from(value: FormatError) -> Self {
        Error::FormatError(value)
    }
}

#[derive(Debug)]
pub struct FoundFile {
    path: String,
    date: Date,
    full_path: PathBuf,
}

impl Display for FoundFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.path, self.date)
    }
}

impl PartialEq for FoundFile {
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path)
    }
}

impl Eq for FoundFile {}

impl PartialOrd for FoundFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.path.partial_cmp(&other.path)
    }
}

impl Ord for FoundFile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.path.cmp(&other.path)
    }
}

impl Hash for FoundFile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

fn scan_dir_and_recurse(dir: &DirEntry, to_keep: &mut BTreeSet<FoundFile>, to_remove: &mut BTreeSet<PathBuf>) -> Result<(), Error> {
    let ty = dir.file_type()?;
    let path = dir.path();
    if ty.is_dir() {
        let dirs = std::fs::read_dir(path)?;
        for dir in dirs {
            let dir = dir?;
            scan_dir_and_recurse(&dir, to_keep, to_remove)?;
        }
        return Ok(());
    }
    let path_str = path.display().to_string();
    let base_path :Vec<&str> = path_str.split('/').next_back().unwrap().split('_').collect();
    let base_path = base_path.split_at(base_path.len()-3).0.join("_");
    let mut paths = path_str.split('_');
    let _ext = paths.next_back();
    let _tm = paths.next_back();
    let Some(date) = paths.next_back() else {
        error!("Error processing path: {path_str}");
        return Ok(());
    };
    let date = BASIC_CALENDAR_DATE.try_from(date)?;

    let found_file = FoundFile {
        path: base_path,
        date,
        full_path: path,
    };

    if to_keep.contains(&found_file) {
        let old = to_keep.take(&found_file).unwrap();
        if old.date < found_file.date {
            debug!("Replacing existing {old} with {found_file}");
            to_keep.insert(found_file);
            to_remove.insert(old.full_path);
        } else {
            debug!("Not replacing existing {old} with {found_file}");
            to_remove.insert(found_file.full_path);
            to_keep.insert(old);
        }
    } else {
        debug!("Found new file {found_file}");
        to_keep.insert(found_file);
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    irox_log::init_console_from_env("CHARTS_LOG");
    let path = "/chonko-1/chartdata/USGS-Topo/28-JAN-2023";
    let dirs = std::fs::read_dir(path)?;

    let mut to_keep: BTreeSet<FoundFile> = BTreeSet::new();
    let mut to_remove: BTreeSet<PathBuf> = BTreeSet::new();

    for dir in dirs {
        let dir = dir?;
        scan_dir_and_recurse(&dir, &mut to_keep, &mut to_remove)?;
    }

    for file in &to_remove {
        info!("Will remove {}", file.display());
        std::fs::remove_file(&file)?;
    }
    info!("Found {} files to keep.", to_keep.len());
    info!("Found {} files to remove.", to_remove.len());

    Ok(())
}
