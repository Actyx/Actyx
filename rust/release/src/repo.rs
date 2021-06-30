#[cfg(not(windows))]
use git2::FetchOptions;
use git2::{Commit, Cred, Oid, PushOptions, RemoteCallbacks, Repository, Signature};
use std::path::{Path, PathBuf};

use crate::{
    changes::{try_change_from_line, Change},
    products::Product,
};

pub struct RepoWrapper(Repository);

impl RepoWrapper {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(Repository::open_from_env()?))
    }
    pub fn workdir(&self) -> anyhow::Result<PathBuf> {
        let wd = self
            .0
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("did not get working dir for repository"))?;

        Ok(wd.to_path_buf().canonicalize()?)
    }
    pub fn head_hash(&self) -> anyhow::Result<Oid> {
        let obj = self.head()?;
        Ok(obj.id())
    }
    pub fn head(&self) -> anyhow::Result<git2::Object> {
        Ok(self.0.revparse_single("HEAD")?)
    }
    pub fn checkout(&self, branch_name: &str, target: &Commit) -> anyhow::Result<()> {
        let _branch = self.0.branch(&*branch_name, target, false)?;
        let branch_ref = format!("refs/heads/{}", branch_name);
        let obj = self.0.revparse_single(&*branch_ref)?;
        self.0.checkout_tree(&obj, None)?;

        self.0.set_head(&*branch_ref)?;
        Ok(())
    }
    pub fn add_file(&self, path: impl AsRef<Path>) -> anyhow::Result<git2::Oid> {
        let mut index = self.0.index()?;
        let path = path.as_ref().canonicalize()?;
        index.add_path(path.strip_prefix(self.workdir()?)?)?;
        let oid = index.write_tree()?;
        Ok(oid)
    }
    pub fn commit(&self, treeish: git2::Oid, message: &str) -> anyhow::Result<git2::Oid> {
        let parent = self.0.head()?.resolve()?.peel_to_commit()?;
        let tree = self.0.find_tree(treeish)?;

        let sig = Signature::now("Actyx Releases", "developer@actyx.com")?;
        let oid = self.0.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])?;
        Ok(oid)
    }
    pub fn push(&self, remote: &str, branch_name: &str) -> anyhow::Result<()> {
        if std::env::var("AZURE_HTTP_USER_AGENT").is_ok() {
            eprintln!("Running inside Azure Pipelines; shelling out to `git`. Output:");
            // `git` is properly set up on Azure Pipelines
            let mut child = std::process::Command::new("git")
                .args(&["push", remote, branch_name])
                .spawn()?;
            anyhow::ensure!(child.wait()?.success());
            // println!(
            //     "###vso[task.setvariable variable=RELEASE_BRANCH;isOutput=true]{}",
            //     branch_name
            // );
            Ok(())
        } else {
            let branch_ref = format!("refs/heads/{}", branch_name);
            let mut remote = self.0.find_remote(remote)?;
            let mut cb = RemoteCallbacks::new();
            cb.credentials(|_, user, _| Cred::ssh_key_from_agent(user.unwrap()));
            let mut opts = PushOptions::new();
            opts.remote_callbacks(cb);
            remote.push(&[format!("{}:{}", branch_ref, branch_ref)], Some(&mut opts))?;
            Ok(())
        }
    }
    #[cfg(not(windows))]
    pub fn head_of_origin_master(&self) -> anyhow::Result<Oid> {
        if std::env::var("AZURE_HTTP_USER_AGENT").is_ok() {
            eprintln!("Running inside Azure Pipelines; shelling out to `git`. Output:");
            // `git` is properly set up on Azure Pipelines
            let mut child = std::process::Command::new("git").args(&["remote", "update"]).spawn()?;
            anyhow::ensure!(child.wait()?.success());
            self.0.find_remote("origin")?;
        } else {
            let mut remote = self.0.find_remote("origin")?;
            let mut cb = RemoteCallbacks::new();
            cb.credentials(|_, user, _| Cred::ssh_key_from_agent(user.unwrap()));
            let mut opts = FetchOptions::new();
            opts.remote_callbacks(cb);
            remote.fetch(&["master"], Some(&mut opts), None)?;
        };
        let head_of_master = self.0.revparse_single("origin/master")?.peel_to_commit()?.id();
        Ok(head_of_master)
    }
}

fn get_commits<'a>(
    repo: &'a Repository,
    from_excl: &Oid,
    to_incl: &Oid,
) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Commit<'a>>>> {
    let mut walk = repo.revwalk().expect("error creating revwalk");
    walk.push_range(format!("{}..{}", from_excl, to_incl).as_str())?;
    Ok(walk.into_iter().map(move |x| {
        let oid = x?;
        Ok(repo.find_commit(oid)?)
    }))
}

pub fn get_changes_for_product(
    repo: &Repository,
    from_excl: &Oid,
    to_incl: &Oid,
    product: &Product,
    commit_ids_to_ignore: &[Oid],
) -> anyhow::Result<Vec<(String, Change)>> {
    let commits = get_commits(repo, from_excl, to_incl)?;
    let mut changes = vec![];
    for commit in commits {
        let commit = commit?;
        if let Some(m) = commit.message() {
            if !commit_ids_to_ignore.contains(&commit.id()) {
                changes.append(
                    &mut m
                        .lines()
                        .filter_map(try_change_from_line)
                        .filter(|c| &c.product == product)
                        .map(|c| (commit.id().to_string(), c))
                        .collect::<Vec<(String, Change)>>(),
                )
            }
        }
    }
    Ok(changes)
}
