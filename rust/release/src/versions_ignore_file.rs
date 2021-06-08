use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

pub struct VersionsIgnoreFile {
    ignore_commit_ids: Vec<String>,
}

impl VersionsIgnoreFile {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let buf = BufReader::new(file);
        let mut ignore_commit_ids = Vec::new();
        for l in buf.lines() {
            let l = l?;
            if !l.starts_with('#') && !l.is_empty() {
                ignore_commit_ids.push(l);
                //versions.push(VersionLine::from_str(&*l)?);
            }
        }
        Ok(Self { ignore_commit_ids })
    }

    pub fn ignore_commit_ids(&self) -> Vec<String> {
        self.ignore_commit_ids.clone()
    }
}
