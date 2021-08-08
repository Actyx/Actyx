use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use git2::Oid;

pub struct VersionsIgnoreFile {
    pub ignore_commit_ids: Vec<Oid>,
}

impl VersionsIgnoreFile {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let buf = BufReader::new(file);
        let mut ignore_commit_ids = Vec::new();
        for l in buf.lines() {
            let l = l?;
            if !l.starts_with('#') && !l.is_empty() {
                let oid = l.parse()?;
                ignore_commit_ids.push(oid);
            }
        }
        Ok(Self { ignore_commit_ids })
    }
}
