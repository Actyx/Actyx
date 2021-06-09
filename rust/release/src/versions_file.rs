use crate::changes::Change;
use crate::products::Product;
use crate::releases::Release;
use crate::repo::get_changes_for_product;
use crate::repo::Hash;
use crate::versions::apply_changes;
use crate::versions_ignore_file::VersionsIgnoreFile;
use anyhow::anyhow;
use anyhow::Context;
use git2::{Oid, Repository};
use itertools::Itertools;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use semver::Version;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BinaryHeap;
use std::fmt;
use std::io::Write;
use std::str;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    str::FromStr,
};
use tempfile::NamedTempFile;

const HEADER: &str = r#"# Last releases of all Actyx products
# Each line contains <release> <commit-hash>
# The machine-readable product names are: actyx, node-manager,
# cli, pond, ts-sdk, rust-sdk, docs, csharp-sdk"#;

pub struct CalculationResult {
    pub prev_commit: Hash,
    pub prev_version: Version,
    pub new_version: Option<Version>,
    pub changes: Vec<Change>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VersionLine {
    pub commit: Hash,
    pub release: Release,
}
impl VersionLine {
    pub fn new(commit: Hash, release: Release) -> Self {
        Self { commit, release }
    }
}
impl PartialOrd for VersionLine {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for VersionLine {
    fn cmp(&self, other: &Self) -> Ordering {
        self.release.cmp(&other.release)
    }
}

impl fmt::Display for VersionLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.release, self.commit)
    }
}

impl FromStr for VersionLine {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        anyhow::ensure!(
            parts.len() >= 2,
            "should be at least two parts in string (found {:?})",
            s
        );

        let commit: Hash = parts[1].to_string().into();
        let release = Release::from_str(parts[0])?;
        Ok(Self { commit, release })
    }
}
pub struct VersionsFile {
    versions: BinaryHeap<VersionLine>,
}

impl fmt::Display for VersionsFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", HEADER)?;
        writeln!(f)?;
        for v in &self.versions() {
            writeln!(f, "{}", v)?;
        }
        Ok(())
    }
}

impl VersionsFile {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let buf = BufReader::new(file);
        let mut versions = BinaryHeap::new();
        for l in buf.lines() {
            let l = l?;
            if !l.starts_with('#') && !l.is_empty() {
                versions.push(VersionLine::from_str(&*l)?);
            }
        }
        Ok(Self { versions })
    }

    pub fn versions(&self) -> Vec<VersionLine> {
        self.versions.clone().into_sorted_vec()
    }
    pub fn persist(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let mut temp = NamedTempFile::new_in(
            path.as_ref()
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Can't get parent of {}", path.as_ref().display()))?,
        )?;
        write!(&mut temp, "{}", self)?;
        temp.flush()?;
        temp.persist(path)?;
        Ok(())
    }

    pub fn add_new_version(&mut self, version: VersionLine) {
        self.versions.push(version)
    }

    pub fn calculate_version(
        &self,
        product: &Product,
        ignore: &VersionsIgnoreFile,
    ) -> anyhow::Result<CalculationResult> {
        let VersionLine {
            commit: last_hash,
            release: last_release,
        } = self
            .versions()
            .into_iter()
            .find(|VersionLine { release, .. }| &release.product == product)
            .ok_or_else(|| anyhow!("did not find past release of {}", product))?;

        let repo = Repository::open_from_env()?;

        let ignore_commit_ids: Vec<Oid> = ignore
            .ignore_commit_ids()
            .iter()
            .map(|spec| {
                repo.revparse_ext(spec)
                    .map(|(obj, _)| obj.id())
                    .with_context(|| format!("interpreting ignore spec {}", spec))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let last_version = last_release.version;
        let mut changes =
            get_changes_for_product(&repo, &last_hash, &Hash("HEAD".into()), product, &ignore_commit_ids)?;

        // We sort the changes by severity (i.e. breaking change first, then feat, etc.). See
        // the PartialOrd implementation for ChangeKind for ordering details.
        //changes.sort_by(|a, b| b.partial_cmp(a).unwrap());
        changes.sort();

        let new_version = apply_changes(&product, &last_version, &changes);

        Ok(CalculationResult {
            prev_commit: last_hash,
            new_version: if last_version == new_version {
                None
            } else {
                Some(new_version)
            },
            prev_version: last_version,
            changes,
        })
    }

    /// Calculates version update (if any) for all products
    pub fn calculate_versions(
        &self,
        ignore: &VersionsIgnoreFile,
    ) -> anyhow::Result<BTreeMap<Product, CalculationResult>> {
        Product::ALL
            .par_iter()
            .map(|p| self.calculate_version(p, ignore).map(|x| (p.clone(), x)))
            .collect()
    }

    /// Calculates all changes between the given version and its predecessor
    pub fn calculate_changes_for_version(
        &self,
        product: &Product,
        version: &Version,
        ignore: &VersionsIgnoreFile,
    ) -> anyhow::Result<Vec<Change>> {
        let all_releases: Vec<_> = self
            .versions()
            .into_iter()
            .filter(|r| &r.release.product == product)
            .collect();
        if all_releases.is_empty() {
            anyhow::bail!("did not find past release of {}", product);
        }

        if all_releases
            .last()
            .map(|VersionLine { release, .. }| &release.version == version)
            .unwrap_or(false)
        {
            anyhow::bail!("no changes since {} is the very first release of {}.", version, product);
        }

        let all_releases: Vec<(_, _)> = all_releases.into_iter().tuple_windows().collect();

        let (this_release, prev_release) = all_releases
            .into_iter()
            .find(|w| &w.0.release.version == version)
            .ok_or(anyhow!(format!("did not find version {} for {}", version, product)))?;

        let repo = Repository::open_from_env()?;
        eprintln!(
            "from {} to {} for {}",
            &prev_release.commit.to_string().as_str(),
            &this_release.commit.to_string().as_str(),
            product,
        );
        let ignore_commit_ids = ignore
            .ignore_commit_ids()
            .iter()
            .map(|spec| {
                repo.revparse_ext(spec)
                    .map(|(obj, _)| obj.id())
                    .with_context(|| format!("interpreting ignore spec {}", spec))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let mut changes = get_changes_for_product(
            &repo,
            &prev_release.commit,
            &this_release.commit,
            product,
            &ignore_commit_ids,
        )?;

        // We sort the changes by severity (i.e. breaking change first, then feat, etc.). See
        // the PartialOrd implementation for ChangeKind for ordering details.
        changes.sort_by(|a, b| b.partial_cmp(a).unwrap());

        Ok(changes)
    }
}

#[test]
fn parse_release_line_test() -> anyhow::Result<()> {
    use crate::products::Product;
    assert_eq!(
        VersionLine::from_str("actyx-1.2.3 commit")?,
        VersionLine::new("commit".to_string().into(), Release::new(Product::Actyx, 1, 2, 3))
    );
    assert_eq!(
        VersionLine::from_str("node-manager-1.2.3 commit")?,
        VersionLine::new("commit".to_string().into(), Release::new(Product::NodeManager, 1, 2, 3))
    );
    assert!(Release::from_str("item-1.2").is_err());
    assert!(Release::from_str("item-1.2.3.4").is_err());
    assert!(Release::from_str("abc item-1.3.4").is_err());
    assert!(Release::from_str("abc").is_err());
    Ok(())
}
