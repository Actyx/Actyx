use anyhow::{anyhow, Error};
use chrono::{TimeZone, Utc};
use clap::Clap;
use repo::RepoWrapper;
use semver::Version;
use std::{fmt::Write, path::PathBuf};
use versions_file::{VersionLine, VersionsFile};
use versions_ignore_file::VersionsIgnoreFile;

mod changes;
mod products;
mod releases;
mod repo;
mod versions;
mod versions_file;
mod versions_ignore_file;

use crate::{products::Product, releases::Release};

#[derive(Clap)]
struct Opts {
    /// Path to persisted version file. Defaults to <repo_root/versions>
    #[clap(short, long, global = true)]
    input: Option<PathBuf>,
    /// Path to persisted versions ignore file. Defaults to <repo_root/versions-ignore>
    #[clap(long, global = true)]
    ignores: Option<PathBuf>,
    #[clap(subcommand)]
    cmd: Command,
}
#[derive(Clap)]
#[clap(version = "1.0", author = "Actyx AG", about = "Releases from Cosmos")]
enum Command {
    /// Computes current version
    Version {
        /// Product (actyx, pond, cli, node-manager, ts-sdk, rust-sdk)
        product: Product,
    },
    /// Computes past version
    Versions {
        /// Product (actyx, pond, cli, node-manager, ts-sdk, rust-sdk)
        product: Product,
        /// Show the git commit hash next to the version
        #[clap(long, short)]
        commits: bool,
    },
    /// Computes changelog
    Changes {
        /// Product (actyx, pond, cli, node-manager, ts-sdk, rust-sdk)
        product: Product,
        /// Specific past version to get changes for (optional)
        version: Option<Version>,
    },
    /// Updates a persisted version file
    Update {
        /// Path to persisted version file. Defaults to stdout if omitted
        #[clap(short, long)]
        output: Option<PathBuf>,
    },
    /// For the current repo, this will calculate the set of version changes
    /// based on the versions persisted in the given input file. If there are
    /// new versions, the following happens:
    ///    1) Create new branch `release/<HEAD ID>`;
    ///    2) Commit changes to input file; the changelog will be placed into
    ///    the commit's message;
    ///    3) Push new branch to `origin`.
    Release {
        /// Print action plan to stdout
        #[clap(long, short)]
        dry_run: bool,
    },
}

fn main() -> Result<(), Error> {
    let opts = Opts::parse();
    let repo = RepoWrapper::new()?;
    let input_file = if let Some(file) = opts.input {
        file
    } else {
        let mut p = repo.workdir()?;
        p.push("versions");
        p
    };
    let ignores_file = if let Some(file) = opts.ignores {
        file
    } else {
        let mut p = repo.workdir()?;
        p.push("versions-ignore");
        p
    };
    let mut version_file = VersionsFile::load(&input_file).map_err(|e| {
        anyhow!(
            "unable to open versions file at {}: {}",
            input_file.to_string_lossy(),
            e
        )
    })?;
    let ignores_file = VersionsIgnoreFile::load(&ignores_file).map_err(|e| {
        anyhow!(
            "unable to open versions-ignore file at {}: {}",
            ignores_file.to_string_lossy(),
            e
        )
    })?;
    match opts.cmd {
        Command::Version { product } => {
            let res = version_file.calculate_version(&product, &ignores_file)?;

            let new_version = res
                .new_version
                .ok_or_else(|| anyhow::anyhow!("No new version found for {}", product))?;
            println!("{}", new_version);
        }
        Command::Versions { product, commits } => {
            let mut releases = version_file
                .versions()
                .into_iter()
                .filter(|VersionLine { release, .. }| release.product == product)
                .peekable();
            anyhow::ensure!(releases.peek().is_some(), "no versions found");

            for v in releases {
                if commits {
                    println!("{} {}", v.release.version, v.commit)
                } else {
                    println!("{}", v.release.version)
                }
            }
        }

        Command::Changes { product, version } => {
            let changes = if let Some(version) = version {
                version_file.calculate_changes_for_version(&product, &version, &ignores_file)
            } else {
                version_file
                    .calculate_version(&product, &ignores_file)
                    .map(|c| c.changes)
            }?;

            anyhow::ensure!(!changes.is_empty(), "No changes found");
            for c in changes {
                println!("{}: {}", c.kind, c.message);
            }
        }
        Command::Update { output } => {
            let new_versions = version_file
                .calculate_versions(&ignores_file)?
                .into_iter()
                .filter(|(_, v)| v.new_version.is_some())
                .collect::<Vec<_>>();

            anyhow::ensure!(!new_versions.is_empty(), "No new versions");
            for (product, v) in new_versions {
                let release = Release {
                    product,
                    version: v.new_version.unwrap(),
                };
                let version = VersionLine::new(repo.head_hash()?, release);
                version_file.add_new_version(version);
            }
            if let Some(output) = output {
                eprintln!("Writing output to \"{}\".", output.display());
                version_file.persist(output)?;
            } else {
                println!("{}", version_file);
            }
        }
        Command::Release { dry_run } => {
            let new_versions = version_file
                .calculate_versions(&ignores_file)?
                .into_iter()
                .filter(|(_, v)| v.new_version.is_some())
                .collect::<Vec<_>>();
            if new_versions.is_empty() {
                eprintln!("No new versions. Nothing to do.");
                return Ok(());
            }

            let mut changelog = String::new();
            // commit versions file with change sets in commit message
            let head = repo.head()?;
            let commit = head.as_commit().unwrap();
            let ts = Utc.timestamp(commit.time().seconds(), 0);
            writeln!(
                changelog,
                r#"Actyx Release
-------------------------
Overview:"#
            )?;
            for (product, v) in &new_versions {
                writeln!(
                    changelog,
                    "  * {}:\t\t{} --> {}",
                    product,
                    v.prev_version,
                    v.new_version.as_ref().unwrap()
                )?;
            }

            writeln!(changelog, "-------------------------")?;
            writeln!(changelog, "Detailed changelog:")?;
            for (product, v) in new_versions {
                let new_version = v.new_version.unwrap();

                writeln!(changelog, "* {}\t\t{}", product, new_version)?;
                for change in v.changes {
                    writeln!(changelog, "    * {}: {}", change.kind, change.message)?;
                }
                let release = Release {
                    product,
                    version: new_version,
                };
                let version = VersionLine::new(repo.head_hash()?, release);
                version_file.add_new_version(version);
            }

            writeln!(changelog, "-------------------------")?;
            // meta
            writeln!(changelog, "Commit of release: {}", head.id())?;
            writeln!(changelog, "Time of release: {}", ts)?;
            let branch_name = format!("release/{}", head.short_id()?.as_str().unwrap());

            if dry_run {
                println!("New versions file:");
                println!("-------------------------");
                println!("{}", version_file);
                println!("-------------------------\n");
                println!("Changelog");
                println!("-------------------------");
                println!("{}", changelog);
                println!("-------------------------\n");
                println!("Branch to create {}", branch_name);
            } else {
                eprint!(
                    "1) Writing new versions to \"{}\" ... ",
                    input_file.display()
                );
                version_file.persist(&input_file)?;
                eprintln!("Done.");

                eprint!("2) git checkout -b {} ... ", branch_name);
                repo.checkout(&*branch_name, &commit)?;
                eprintln!("Done.");

                eprint!("3) git add \"{}\" ... ", input_file.display());
                let oid = repo.add_file(&input_file)?;
                eprintln!("Done.");

                eprint!("3) git commit ... ");
                let oid = repo.commit(oid, &*changelog)?;
                eprintln!("Done. ({})", oid);

                eprint!("4) git push origin/{} ... ", branch_name);
                if std::env::var("AZURE_HTTP_USER_AGENT").is_ok() {
                    eprintln!("Running inside Azure Pipelines; shelling out to `git`. Output:");
                    // `git` is properly set up on Azure Pipelines
                    let mut child = std::process::Command::new("git")
                        .args(&["push", "origin", &*branch_name])
                        .spawn()?;
                    anyhow::ensure!(child.wait()?.success());
                    // println!(
                    //     "###vso[task.setvariable variable=RELEASE_BRANCH;isOutput=true]{}",
                    //     branch_name
                    // );
                } else {
                    repo.push("origin", &*branch_name)?;
                }
                eprintln!("Done.");
            }
        }
    }
    Ok(())
}
