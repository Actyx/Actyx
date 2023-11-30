use anyhow::{bail, Context, Error};
use chrono::{TimeZone, Utc};
use clap::Parser;
use repo::RepoWrapper;
use semver::Version;
use std::{
    env,
    fmt::Write,
    fs::OpenOptions,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use toml_edit::Document;
use versions_file::{VersionLine, VersionsFile};
use versions_ignore_file::VersionsIgnoreFile;

mod changes;
mod os_arch;
mod products;
mod publisher;
mod releases;
mod repo;
mod util;
mod versions;
mod versions_file;
mod versions_ignore_file;

#[cfg(not(windows))]
use crate::{os_arch::OsArch, publisher::Publisher};
use crate::{products::Product, releases::Release};
#[cfg(not(windows))]
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
#[cfg(not(windows))]
use tempfile::tempdir;

#[derive(Parser)]
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
#[derive(Parser)]
#[clap(version = "1.0", author, about)]
enum Command {
    /// Computes current version
    Version {
        /// Product (ax, ax_core, actyx, pond, cli, node-manager, ts-sdk, rust-sdk)
        product: Product,
    },
    /// Computes past versions
    Versions {
        /// Product (ax, ax_core, actyx, pond, cli, node-manager, ts-sdk, rust-sdk)
        product: Product,
        /// Show the git commit hash next to the version
        #[clap(long, short)]
        commits: bool,
    },
    /// Computes the ACTYX_VERSION string for the given product.  If there's a
    /// pending change for a product, this command will calculate and emit the
    /// NEW version. Otherwise it falls back to the last released one; if the
    /// release hash is not equal to HEAD, this will append `_dev` to the semver
    /// version.
    GetActyxVersion { product: Product },
    /// Computes changelog
    Changes {
        /// Product (actyx, pond, cli, node-manager, ts-sdk, rust-sdk)
        product: Product,
        /// Specific past version to get changes for (optional)
        version: Option<Version>,
        /// Show the git commit hash next to the change
        #[clap(long, short)]
        commits: bool,
    },
    /// Retrieve full history information for website in one go
    History,
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
    /// Makes sure all released versions of a given product are released
    #[cfg(not(windows))]
    Publish {
        product: Product,
        /// Don't publish
        #[clap(long, short)]
        dry_run: bool,
        /// Force running even when running not on HEAD of master
        #[clap(long, short)]
        force: bool,
        /// Ignore errors
        #[clap(long)]
        ignore_errors: bool,
    },
}

fn main() -> Result<(), Error> {
    env_logger::try_init()?;
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
    let mut version_file = VersionsFile::load(&input_file)
        .with_context(|| format!("Opening versions file at {}", input_file.display()))?;
    let ignores_file = VersionsIgnoreFile::load(&ignores_file)
        .with_context(|| format!("Opening versions-ignore file at {}", ignores_file.display()))?;
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

        Command::GetActyxVersion { product } => {
            let head_commit_id = repo.head()?.id();
            let new_version = version_file.calculate_version(&product, &ignores_file)?.new_version;
            if let Some(version) = new_version {
                println!("{}-{}", version, head_commit_id);
            } else {
                let v = version_file
                    .versions()
                    .into_iter()
                    .find(|VersionLine { release, .. }| release.product == product)
                    .ok_or_else(|| anyhow::anyhow!("No release found for {}", product))?;

                if head_commit_id == v.commit {
                    println!("{}-{}", v.release.version, v.commit)
                } else {
                    let is_js = matches!(product, Product::NodeManager | Product::Pond | Product::TsSdk);
                    // npm is serious about semver
                    let delimiter = if is_js { '-' } else { '_' };
                    println!("{}{}dev-{}", v.release.version, delimiter, head_commit_id)
                }
            }
        }

        Command::Changes {
            product,
            version,
            commits,
        } => {
            let changes = if let Some(version) = version {
                version_file.calculate_changes_for_version(&product, &version, &ignores_file)
            } else {
                version_file
                    .calculate_version(&product, &ignores_file)
                    .map(|c| c.changes)
            }?;

            anyhow::ensure!(!changes.is_empty(), "No changes found");
            for (hash, c) in changes {
                if commits {
                    println!("{}: {} [{}]", c.kind, c.message, hash);
                } else {
                    println!("{}: {}", c.kind, c.message);
                }
            }
        }

        Command::History => {
            let history = version_file.history(&ignores_file)?;
            println!("{}", serde_json::to_string(&history)?);
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
            let ts = Utc
                .timestamp_opt(commit.time().seconds(), 0)
                .single()
                .expect("a single timestamp");
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
            for (product, v) in new_versions.clone() {
                let new_version = v.new_version.unwrap();

                writeln!(changelog, "* {}\t\t{}", product, new_version)?;
                for (commit, change) in v.changes {
                    writeln!(changelog, "    * {}: {} [{}]", change.kind, change.message, commit)?;
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
            let branch_name = format!("release/{}", head.id());

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
                // This pathing abuses the fact that we control where this is run from: CI
                // and in CI this is *usually* run in the rust/release path
                let ax_cargo = PathBuf::from("../actyx/ax/Cargo.toml").canonicalize()?;
                let ax_core_cargo = PathBuf::from("../actyx/ax-core/Cargo.toml").canonicalize()?;
                for (product, v) in new_versions {
                    let new_version = v.new_version.unwrap();
                    match product {
                        Product::Ax => {
                            eprint!("0.1) Writing new version to \"{}\" ... ", ax_cargo.display());
                            update_package_version(&ax_cargo, &new_version)?;
                            std::fs::write(
                                PathBuf::from("../actyx/ax-core/node/version.rs").canonicalize()?,
                                format!(
                                    r#"/// The databank version.
///
/// This version is kept automatically!
pub const DATABANK_VERSION: &str = "{}";"#,
                                    new_version
                                ),
                            )?;
                            repo.add_file(&ax_cargo)?;
                        }
                        Product::AxCore => {
                            eprint!("0.2) Writing new version to \"{}\" ... ", ax_core_cargo.display());
                            update_package_version(&ax_core_cargo, &new_version)?;
                            repo.add_file(&ax_core_cargo)?;
                        }
                        // We're not updating TOMLs for anything else
                        _ => (),
                    };
                }

                eprint!("1) Writing new versions to \"{}\" ... ", input_file.display());
                version_file.persist(&input_file)?;
                eprintln!("Done.");

                eprint!("2) git checkout -b {} ... ", branch_name);
                if repo.branch_exists(&branch_name)? {
                    eprintln!("Already exists. Exiting");
                    return Ok(());
                } else {
                    repo.checkout(&branch_name, commit)?;
                    eprintln!("Done.");
                }

                eprint!("3) git add \"{}\" ... ", input_file.display());
                let oid = repo.add_file(&input_file)?;
                eprintln!("Done.");

                eprint!("3) git commit ... ");
                let oid = repo.commit(oid, &changelog)?;
                eprintln!("Done. ({})", oid);

                eprint!("4) git push origin/{} ... ", branch_name);
                repo.push("origin", &branch_name)?;
                eprintln!("Done.");

                // This is used in GitHub Actions to create a new output
                // https://docs.github.com/en/actions/using-workflows/workflow-commands-for-github-actions#setting-an-output-parameter
                if let Ok(env_file) = env::var("GITHUB_OUTPUT") {
                    use std::io::Write;
                    let mut file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&env_file)
                        .unwrap_or_else(|_| {
                            panic!(
                                "The file \"{}\" should be able to be created, opened and written to",
                                &env_file
                            )
                        });
                    writeln!(file, "RELEASE_BRANCH={}", branch_name)
                        .unwrap_or_else(|_| panic!("The file \"{}\" should be writeable", &env_file));
                }
            }
        }
        #[cfg(not(windows))]
        Command::Publish {
            product,
            dry_run,
            force,
            ignore_errors,
        } => {
            let head_of_origin_master = repo.head_of_origin_master()?;
            let head = repo.head_hash()?;
            anyhow::ensure!(
                dry_run || (head_of_origin_master == head) || force,
                "Not up to date with origin/master \
                (current head {}, head of origin/master {}). \
                Use `--force` to override.",
                head,
                head_of_origin_master
            );

            let versions = version_file
                .versions()
                .into_iter()
                .filter(|VersionLine { release, .. }| release.product == product);
            eprintln!("Checking releases for {}", product);

            // CI has too many cores
            rayon::ThreadPoolBuilder::new().num_threads(6).build_global().unwrap();
            for (idx, VersionLine { commit, release }) in versions.enumerate() {
                if ignores_file.ignore_commit_ids.contains(&commit) {
                    println!("  {} ({}) ignored", release, commit);
                } else {
                    let tmp = tempdir()?;
                    log::debug!("Temp dir for {}: {}", release, tmp.path().display());

                    let needed_write = AtomicBool::new(false);
                    let out = OsArch::all()
                        .par_iter()
                        .map(|os_arch| {
                            log::debug!("creating publisher for arch {}", os_arch);
                            Publisher::new(&release, &commit, *os_arch, idx == 0).and_then(|p| {
                                p.into_iter()
                                    .map(|mut p| {
                                        let mut out = String::new();
                                        let source_exists = p.source_exists()?;
                                        let target_exists = p.target_exists()?;
                                        if target_exists {
                                            writeln!(&mut out, "    [OK] {} already exists.", p.target)?;
                                        } else if source_exists {
                                            needed_write.store(true, Ordering::Relaxed);
                                            log::debug!("creating release artifact in dir {}", tmp.path().display());
                                            p.create_release_artifact(tmp.path()).context(format!(
                                                "creating release artifact at {}",
                                                tmp.path().display()
                                            ))?;
                                            if dry_run {
                                                writeln!(&mut out, "    [DRY RUN] Create and publish {}", p.target)?;
                                            } else {
                                                log::debug!("starting publishing");
                                                p.publish().context("publishing")?;
                                                log::debug!("finished publishing");
                                                writeln!(&mut out, "    [NEW] {}", p.target)?;
                                            }
                                        } else {
                                            if !ignore_errors && !dry_run {
                                                anyhow::bail!("    [ERR] Source \"{}\" does NOT exist.", p.source);
                                            }
                                            writeln!(&mut out, "    [ERR] Source \"{}\" does NOT exist.", p.source)?;
                                        }
                                        Ok(out)
                                    })
                                    .collect::<anyhow::Result<Vec<String>>>()
                            })
                        })
                        .collect::<anyhow::Result<Vec<_>>>()?
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join("");
                    println!("  {} ({}) .. ", release, commit);
                    println!("{}", out);
                    if !needed_write.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

fn update_package_version(path: &PathBuf, version: &Version) -> Result<(), Error> {
    // Read the toml
    let cargo_toml_contents = if !path.is_file() {
        bail!("{:?} is not a file", path);
    } else {
        std::fs::read_to_string(path)?
    };
    // Parse it
    let mut cargo_toml = cargo_toml_contents.parse::<Document>()?;
    // Update the value
    cargo_toml["package"]["version"] = toml_edit::value(version.to_string());
    // Write it back
    std::fs::write(path, cargo_toml.to_string())?;
    Ok(())
}
