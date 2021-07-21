use crate::{
    os_arch::{Arch, OsArch, OS},
    products::Product,
    releases::Release,
};
use anyhow::Context;
use flate2::{write::GzEncoder, Compression};
use git2::Oid;
use serde::Deserialize;
use std::{
    fmt,
    fs::File,
    os::unix::prelude::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::tempdir_in;
use zip::ZipWriter;

#[derive(Debug, Clone)]
pub enum SourceType {
    // Path describing an artifact on the Artifacts blob store
    Blob(String),
    // Docker manifest tag, e.g. docker.io/actyx/actyx:latest
    Docker {
        registry: String,
        repository: String,
        tag: String,
    },
}
#[derive(Debug, Clone)]
pub struct SourceArtifact {
    release: Release,
    os_arch: OsArch,
    r#type: SourceType,
}

impl fmt::Display for SourceArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.r#type {
            SourceType::Blob(s) => write!(f, "Source Blob \"{}\" for {} ({})", s, self.release, self.os_arch),
            SourceType::Docker {
                registry,
                repository,
                tag,
            } => write!(
                f,
                "Source Docker \"{}/{}:{}\" for {} ({})",
                registry, repository, tag, self.release, self.os_arch
            ),
        }
    }
}
#[derive(Debug, Clone)]
pub enum TargetArtifact {
    Blob {
        /// Preprocessing step to create the target out of a source
        pre_processing: PreProcessing,
        /// Target file name
        file_name: String,
        /// If created, points to the target file. Initially empty
        local_result: Option<PathBuf>,
    },
    Docker {
        registry: String,
        repository: String,
        tag: String,
        manifest: DockerInspectResponse,
    },
}
impl fmt::Display for TargetArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            TargetArtifact::Blob { file_name, .. } => write!(f, "Target Blob \"{}\"", file_name),
            TargetArtifact::Docker { repository, tag, .. } => write!(f, "Target Docker \"{}:{}\"", repository, tag),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Publisher {
    pub source: SourceArtifact,
    pub target: TargetArtifact,
}
impl Publisher {
    pub fn new(release: &Release, commit: &Oid, os_arch: OsArch, is_newest: bool) -> anyhow::Result<Vec<Self>> {
        Ok(mk_blob_tuples(release, commit, os_arch)
            .into_iter()
            .chain(mk_docker_tuples(release, commit, os_arch, is_newest)?.into_iter())
            .map(|(source, target)| Self { source, target })
            .collect())
    }
    pub fn source_exists(&self) -> anyhow::Result<bool> {
        log::debug!("checking if source {} exists for target {}", self.source, self.target);
        match &self.source.r#type {
            SourceType::Blob(s) => blob_exists(Container::Artifacts, &*s),
            SourceType::Docker {
                registry,
                tag,
                repository,
            } => docker_manifest_exists(&*format!("{}/{}:{}", registry, repository, tag), None),
        }
    }
    pub fn target_exists(&self) -> anyhow::Result<bool> {
        log::debug!("checking if target {} exists for source {}", self.target, self.source);
        match &self.target {
            TargetArtifact::Blob { file_name: s, .. } => blob_exists(Container::Releases, &*s),
            TargetArtifact::Docker {
                registry,
                manifest,
                repository,
                tag,
            } => docker_manifest_exists(&*format!("{}/{}:{}", registry, repository, tag), Some(manifest)),
        }
    }
    pub fn create_release_artifact(&mut self, in_dir: impl AsRef<Path>) -> anyhow::Result<()> {
        match &mut self.target {
            TargetArtifact::Blob {
                pre_processing,
                file_name,
                local_result,
            } => {
                let source = if let SourceArtifact {
                    r#type: SourceType::Blob(source),
                    ..
                } = &self.source
                {
                    &*source
                } else {
                    anyhow::bail!("Tried creating {:?} from {:?}", self.target, self.source);
                };
                // create a unique directory within `in_dir` to download the
                // artifact into. The source artifacts are not uniquely named.
                let tmp = tempdir_in(&in_dir)?.into_path();
                let source_file = blob_download(source, tmp)?;
                {
                    // Set executable bit on source file. That mostly always
                    // what we want.
                    let mut perms = std::fs::metadata(&source_file)?.permissions();
                    let mode = perms.mode() | 0o111;
                    perms.set_mode(mode);
                    std::fs::set_permissions(&source_file, perms)
                        .with_context(|| format!("Setting permissions for {} to {}", source_file.display(), mode))?;
                }
                let processed = match pre_processing {
                    PreProcessing::TarGz {
                        binary_name: target_name,
                    } => {
                        let out = in_dir.as_ref().join(file_name);
                        package_tar_gz(&source_file, &out, target_name.as_deref())?;
                        out
                    }
                    PreProcessing::Zip {
                        binary_name: target_name,
                    } => {
                        let out = in_dir.as_ref().join(file_name);

                        package_zip(&source_file, &out, target_name.as_deref())?;
                        out
                    }
                    PreProcessing::None => source_file,
                };
                log::info!("Created {}", processed.display());
                local_result.replace(processed);
                Ok(())
            }
            TargetArtifact::Docker {
                registry,
                repository,
                tag,
                manifest,
            } => {
                if let SourceArtifact {
                    r#type:
                        SourceType::Docker {
                            repository: source_repository,
                            ..
                        },
                    ..
                } = &self.source
                {
                    docker_manifest_create(manifest, registry, source_repository, repository, tag)
                } else {
                    unreachable!()
                }
            }
        }
    }
    pub fn publish(&self) -> anyhow::Result<()> {
        match &self.target {
            TargetArtifact::Blob {
                file_name,
                local_result,
                ..
            } => {
                let local = local_result
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("No local file created!"))?;
                blob_upload(local, &*file_name)
            }
            TargetArtifact::Docker {
                tag,
                registry,
                repository,
                ..
            } => docker_manifest_push(&*format!("{}/{}:{}", registry, repository, tag)),
        }
    }
}

pub enum Container {
    Artifacts,
    Releases,
}
impl fmt::Display for Container {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Container::Artifacts => "artifacts",
                Container::Releases => "releases",
            }
        )
    }
}

fn mk_docker_tuples(
    release: &Release,
    hash: &Oid,
    os_arch: OsArch,
    is_newest: bool,
) -> anyhow::Result<Vec<(SourceArtifact, TargetArtifact)>> {
    if release.product == Product::Actyx && os_arch.arch == Arch::x86_64 && os_arch.os == OS::linux {
        // Multiarch image, so just do it once
        let mut out = vec![];
        let registry = "docker.io".to_string();
        let repository = "actyx/cosmos".to_string();
        let tag = format!("actyx-{}", hash);
        let manifest = docker_manifest_inspect(&*format!("{}:{}", repository, tag))?;
        let source = SourceArtifact {
            os_arch,
            release: release.clone(),
            r#type: SourceType::Docker {
                registry: registry.clone(),
                repository,
                tag,
            },
        };
        if is_newest {
            out.push((
                source.clone(),
                TargetArtifact::Docker {
                    registry: registry.clone(),
                    repository: "actyx/actyx".to_string(),
                    tag: "latest".to_string(),
                    manifest: manifest.clone(),
                },
            ));
        }
        out.push((
            source,
            TargetArtifact::Docker {
                registry,
                repository: "actyx/actyx".to_string(),
                tag: release.version.to_string(),
                manifest,
            },
        ));
        Ok(out)
    } else {
        Ok(vec![])
    }
}

/// Maps a source artifact (m) to a target artifact (n).
fn mk_blob_tuples(release: &Release, hash: &Oid, os_arch: OsArch) -> Vec<(SourceArtifact, TargetArtifact)> {
    let mut out = vec![];

    let Release { product, version } = &release;
    // The architecture descriptor are not uniform across all target platforms. We try to go with
    // the "idiomatic" one for each one. This is why we change it in the following sometimes.
    match (product, os_arch.os) {
        (Product::Actyx, OS::android) => {
            out.push((
                format!("{}-binaries/actyx.apk", os_arch.os,),
                TargetArtifact::Blob {
                    pre_processing: PreProcessing::None,
                    file_name: format!("Actyx-{}.apk", version),
                    local_result: None,
                },
            ));
        }
        (Product::Actyx, OS::windows) => {
            if os_arch.arch == Arch::x86_64 {
                out.push((
                    format!("{}-binaries/{}/actyx-x64.msi", os_arch.os, os_arch),
                    TargetArtifact::Blob {
                        pre_processing: PreProcessing::None,
                        file_name: format!("actyx-{}-x64.msi", version),
                        local_result: None,
                    },
                ));
            }
        }
        (Product::Actyx, OS::linux) => {
            let output_arch = match os_arch.arch {
                Arch::x86_64 => "amd64",
                Arch::aarch64 => "arm64",
                Arch::armv7 => "armhf",
                Arch::arm => "arm",
                _ => unreachable!(),
            };
            out.push((
                format!("{}-binaries/{}/actyx-linux", os_arch.os, os_arch),
                TargetArtifact::Blob {
                    pre_processing: PreProcessing::TarGz {
                        binary_name: Some("actyx".into()),
                    },
                    file_name: format!("actyx-{}-linux-{}.tar.gz", version, output_arch),
                    local_result: None,
                },
            ));
        }
        (Product::Actyx, OS::macos) => {
            let output_arch = match os_arch.arch {
                Arch::x86_64 => "intel",
                Arch::aarch64 => "arm",
                _ => unreachable!(),
            };
            out.push((
                format!("{}-binaries/{}/actyx-linux", os_arch.os, os_arch),
                TargetArtifact::Blob {
                    pre_processing: PreProcessing::Zip {
                        binary_name: Some("actyx".into()),
                    },
                    file_name: format!("actyx-{}-macos-{}.zip", version, output_arch),
                    local_result: None,
                },
            ));
        }
        (Product::Actyx, _) => {}
        (Product::Cli, OS::windows) => {
            if os_arch.arch == Arch::x86_64 {
                out.push((
                    format!("{}-binaries/{}/ax.exe", os_arch.os, os_arch),
                    TargetArtifact::Blob {
                        pre_processing: PreProcessing::Zip { binary_name: None },
                        file_name: format!("actyx-cli-{}-windows-x64.zip", version),
                        local_result: None,
                    },
                ));
            }
        }
        (Product::Cli, OS::linux) => {
            let output_arch = match os_arch.arch {
                Arch::x86_64 => "amd64",
                Arch::aarch64 => "arm64",
                Arch::armv7 => "armhf",
                Arch::arm => "arm",
                _ => unreachable!(),
            };
            out.push((
                format!("{}-binaries/{}/ax", os_arch.os, os_arch),
                TargetArtifact::Blob {
                    pre_processing: PreProcessing::TarGz { binary_name: None },
                    file_name: format!("actyx-cli-{}-linux-{}.tar.gz", version, output_arch),
                    local_result: None,
                },
            ));
        }
        (Product::Cli, OS::macos) => {
            let output_arch = match os_arch.arch {
                Arch::x86_64 => "intel",
                Arch::aarch64 => "arm",
                _ => unreachable!(),
            };
            out.push((
                format!("{}-binaries/{}/ax", os_arch.os, os_arch),
                TargetArtifact::Blob {
                    pre_processing: PreProcessing::Zip { binary_name: None },
                    file_name: format!("actyx-cli-{}-macos-{}.zip", version, output_arch),
                    local_result: None,
                },
            ));
        }
        (Product::Cli, _) => {}
        (Product::NodeManager, OS::linux) => {
            if matches!(os_arch.arch, Arch::x86_64) {
                out.push((
                    "node-manager-linux/actyx-node-manager-amd64.deb".to_string(),
                    TargetArtifact::Blob {
                        pre_processing: PreProcessing::None,
                        file_name: format!("actyx-node-manager-{}-amd64.deb", version),
                        local_result: None,
                    },
                ));
                out.push((
                    "node-manager-linux/actyx-node-manager-x86_64.rpm".to_string(),
                    TargetArtifact::Blob {
                        pre_processing: PreProcessing::None,
                        file_name: format!("actyx-node-manager-{}-x86_64.rpm", version),
                        local_result: None,
                    },
                ));
            }
        }
        (Product::NodeManager, OS::windows) => {
            if matches!(os_arch.arch, Arch::x86_64) {
                out.push((
                    "node-manager-win/actyx-node-manager-windows-x64.msi".to_string(),
                    TargetArtifact::Blob {
                        pre_processing: PreProcessing::None,
                        file_name: format!("actyx-node-manager-{}-x64.msi", version),
                        local_result: None,
                    },
                ));
            }
        }
        (Product::NodeManager, OS::macos) => {
            if matches!(os_arch.arch, Arch::x86_64) {
                //let output_arch = match os_arch.arch {
                //    Arch::x86_64 => "intel",
                //    Arch::aarch64 => "arm",
                //    _ => unreachable!(),
                //};
                out.push((
                    "node-manager-mac/ActyxNodeManager-x64.dmg".to_string(),
                    TargetArtifact::Blob {
                        pre_processing: PreProcessing::None,
                        file_name: format!("ActyxNodeManager-{}.dmg", version),
                        local_result: None,
                    },
                ));
            }
        }
        (Product::NodeManager, _) => {}
        (Product::Pond, _) => {}
        (Product::TsSdk, _) => {}
        (Product::RustSdk, _) => {}
        (Product::CSharpSdk, _) => {}
    };

    out.into_iter()
        .map(|(src, target)| {
            (
                SourceArtifact {
                    os_arch,
                    release: release.clone(),
                    r#type: SourceType::Blob(format!("{}/{}", hash, src)),
                },
                target,
            )
        })
        .collect()
}

#[derive(Debug, Clone)]
pub enum PreProcessing {
    TarGz {
        /// Indicates whether the file name of the binary to be packed should be
        /// changed
        binary_name: Option<String>,
    },
    Zip {
        /// Indicates whether the file name of the binary to be packed should be
        /// changed
        binary_name: Option<String>,
    },
    None,
}

fn blob_exists(container: Container, name: &str) -> anyhow::Result<bool> {
    log::debug!("checking if {} exists in container {}", name, container);
    let args = [
        "storage",
        "blob",
        "exists",
        "--account-name",
        "axartifacts",
        "--container",
        &*container.to_string(),
        "--name",
        name,
    ];
    let out = Command::new("az")
        .args(&args)
        .output()
        .context(format!("running az {:?}", args))?;
    let stdout = String::from_utf8(out.stdout).context("decoding az stdout")?;
    let stderr = String::from_utf8(out.stderr).context("decoding az stderr")?;
    log::trace!("stdout {}", stdout);
    log::trace!("stderr {}", stderr);
    // Always returns 0, even if the blob doesn't exist.
    anyhow::ensure!(out.status.success(), "stdout: {}, stderr: {}", stdout, stderr);
    // Output:
    // {
    //   "exists": false | true
    // }
    if stdout.contains("true") {
        log::debug!("blob {} exists", name);
        Ok(true)
    } else {
        log::debug!("blob {} does not exist", name);
        Ok(false)
    }
}
fn blob_download(name: &str, in_dir: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let file_name = name.split('/').last().unwrap();
    let out_file = in_dir.as_ref().join(file_name);
    let args = [
        "storage",
        "blob",
        "download",
        "--account-name",
        "axartifacts",
        "--container-name",
        &*Container::Artifacts.to_string(),
        "--name",
        name,
        "--file",
        &*format!("{}", out_file.display()),
    ];
    let out = Command::new("az")
        .args(&args)
        .output()
        .context(format!("running az {:?}", args))?;
    let stdout = String::from_utf8(out.stdout).context("decoding az stdout")?;
    let stderr = String::from_utf8(out.stderr).context("decoding az stderr")?;
    log::trace!("stdout {}", stdout);
    log::trace!("stderr {}", stderr);
    anyhow::ensure!(out.status.success(), "stdout: {}, stderr: {}", stdout, stderr);
    Ok(out_file)
}

fn blob_upload(source_file: impl AsRef<Path>, name: &str) -> anyhow::Result<()> {
    let args = [
        "storage",
        "blob",
        "upload",
        "--account-name",
        "axartifacts",
        "--container-name",
        &*Container::Releases.to_string(),
        "--name",
        name,
        "--file",
        &*format!("{}", source_file.as_ref().display()),
    ];
    let out = Command::new("az")
        .args(&args)
        .output()
        .context(format!("running az {:?}", args))?;

    let stdout = String::from_utf8(out.stdout).context("decoding az stdout")?;
    let stderr = String::from_utf8(out.stderr).context("decoding az stderr")?;
    log::trace!("stdout {}", stdout);
    log::trace!("stderr {}", stderr);
    anyhow::ensure!(
        out.status.success(),
        "command: az {:?},\nstdout: {},\nstderr: {}",
        args,
        stdout,
        stderr
    );
    Ok(())
}

fn package_tar_gz(source: impl AsRef<Path>, target: impl AsRef<Path>, binary_name: Option<&str>) -> anyhow::Result<()> {
    let output =
        File::create(target.as_ref()).context(format!("creating .tar.gz output file for {:?}", target.as_ref()))?;
    let name = binary_name
        .or_else(|| source.as_ref().file_name().map(|x| x.to_str().unwrap()))
        .unwrap();

    let enc = GzEncoder::new(output, Compression::best());
    let mut tar = tar::Builder::new(enc);
    tar.append_path_with_name(&source, &name).context(format!(
        "appending path {:?} to .tar.gz target {}",
        source.as_ref(),
        name
    ))?;
    Ok(())
}

fn package_zip(source: impl AsRef<Path>, target: impl AsRef<Path>, binary_name: Option<&str>) -> anyhow::Result<()> {
    let name = binary_name
        .or_else(|| source.as_ref().file_name().map(|x| x.to_str().unwrap()))
        .unwrap();
    let mut source = File::open(&source)?;
    let out = File::create(target)?;
    let mut zip = ZipWriter::new(out);
    let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file(name, options)?;

    std::io::copy(&mut source, &mut zip)?;
    zip.finish()?;
    Ok(())
}

/// Checks whether a manifest exists for the given `tag`. If the manifest
/// exists, and an additional `manifest` is given, the existing manifest is
/// checked for equality with that. That is useful for example when one wants to
/// make sure that a re-usable tag points to the right manifest (like `latest`).
fn docker_manifest_exists(tag: &str, manifest: Option<&DockerInspectResponse>) -> anyhow::Result<bool> {
    // Remove locally, so we make sure that we pull the latest manifest for that
    // tag from the repository
    docker_manifest_rm(tag)?;
    Ok(if let Ok(existing_manifest) = docker_manifest_inspect(tag) {
        if let Some(m) = manifest {
            m == &existing_manifest
        } else {
            true
        }
    } else {
        false
    })
}

fn docker_manifest_inspect(tag: &str) -> anyhow::Result<DockerInspectResponse> {
    let args = ["manifest", "inspect", tag];
    let cmd = Command::new("docker")
        .args(&args)
        .output()
        .context(format!("running docker {:?}", args))?;
    anyhow::ensure!(cmd.status.success(), "Error inspecting manifest for {}", tag);
    let mut out: DockerInspectResponse = serde_json::from_slice(&cmd.stdout[..])?;
    out.manifests.sort();
    Ok(out)
}
fn docker_manifest_rm(target: &str) -> anyhow::Result<()> {
    let args = ["manifest", "rm", &*target];

    let cmd = Command::new("docker")
        .args(&args)
        .output()
        .context(format!("running docker {:?}", args))?;

    let stderr = String::from_utf8(cmd.stderr)?;
    anyhow::ensure!(
        cmd.status.success() || stderr.contains("No such manifest"),
        "Error running `docker {:?}` for {}\nstdout: {}\nstderr: {}",
        args,
        target,
        String::from_utf8(cmd.stdout)?,
        stderr
    );
    Ok(())
}
fn docker_manifest_create(
    source_manifest: &DockerInspectResponse,
    registry: &str,
    source_repository: &str,
    target_repository: &str,
    tag: &str,
) -> anyhow::Result<()> {
    let target = format!("{}/{}:{}", registry, target_repository, tag);
    // Make sure the manifest is removed locally, otherwise we can't
    // (re)create it
    docker_manifest_rm(&*target)?;

    // Now the actual manifest creation
    let args = vec!["manifest".to_string(), "create".to_string(), target.clone()]
        .into_iter()
        .chain(
            source_manifest
                .manifests
                .iter()
                .map(|x| format!("{}@{}", source_repository, x.digest)),
        )
        .collect::<Vec<String>>();
    let cmd = Command::new("docker")
        .args(&args)
        .output()
        .context(format!("running docker {:?}", args))?;

    anyhow::ensure!(
        cmd.status.success(),
        "Error running `docker {:?}` for {}\nstdout: {}\nstderr: {}",
        args,
        target,
        String::from_utf8(cmd.stdout)?,
        String::from_utf8(cmd.stderr)?
    );

    Ok(())
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct DockerInspectResponse {
    schema_version: u32,
    media_type: String,
    manifests: Vec<DockerManifest>,
}
#[derive(Deserialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct DockerManifest {
    media_type: String,
    size: u64,
    digest: String,
    platform: DockerManifestPlatform,
}
#[derive(Deserialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct DockerManifestPlatform {
    architecture: String,
    os: String,
    variant: Option<String>,
}
fn docker_manifest_push(target: &str) -> anyhow::Result<()> {
    let args = ["manifest", "push", target];
    log::debug!("running docker {:?}", args);
    let cmd = Command::new("docker")
        .args(&args)
        .output()
        .context(format!("running docker {:?}", args))?;
    anyhow::ensure!(
        cmd.status.success(),
        "Error running `docker {:?}` for {}\nstdout: {}\nstderr: {}",
        args,
        target,
        String::from_utf8(cmd.stdout)?,
        String::from_utf8(cmd.stderr)?
    );
    Ok(())
}
