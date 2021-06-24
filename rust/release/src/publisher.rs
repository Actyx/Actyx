use crate::{
    os_arch::{Arch, OsArch, OS},
    products::Product,
    releases::Release,
};
use flate2::{write::GzEncoder, Compression};
use git2::Oid;
use std::{
    fmt,
    fs::File,
    path::{Path, PathBuf},
    process::Command,
};
use zip::ZipWriter;

#[derive(Debug, Clone)]
pub enum SourceType {
    // Path describing an artifact on the Artifacts blob store
    Blob(String),
    // Docker image identifier, e.g. docker.io/actyx/actyx:latest
    Docker(String),
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
            SourceType::Docker(s) => write!(f, "Source Docker \"{}\" for {} ({})", s, self.release, self.os_arch),
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
    Docker(String),
}
impl fmt::Display for TargetArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            TargetArtifact::Blob { file_name, .. } => write!(f, "Target Blob \"{}\"", file_name),
            TargetArtifact::Docker(s) => write!(f, "Target Docker \"{}\"", s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Publisher {
    pub source: SourceArtifact,
    pub target: TargetArtifact,
}
impl Publisher {
    pub fn new(release: &Release, commit: &Oid, os_arch: OsArch, is_newest: bool) -> Vec<Self> {
        mk_blob_tuples(release, commit, os_arch)
            .into_iter()
            .chain(mk_docker_tuples(release, commit, os_arch, is_newest).into_iter())
            .map(|(source, target)| Self { source, target })
            .collect()
    }
    pub fn source_exists(&self) -> anyhow::Result<bool> {
        match &self.source.r#type {
            SourceType::Blob(s) => blob_exists(Container::Artifacts, &*s),
            SourceType::Docker(s) => docker_image_exists(&*s),
        }
    }
    pub fn target_exists(&self) -> anyhow::Result<bool> {
        match &self.target {
            TargetArtifact::Blob { file_name: s, .. } => blob_exists(Container::Releases, &*s),
            TargetArtifact::Docker(s) => docker_image_exists(&*s),
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
                let source_file = blob_download(source, &in_dir)?;
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
            TargetArtifact::Docker(s) => {
                let source = if let SourceArtifact {
                    r#type: SourceType::Docker(source),
                    ..
                } = &self.source
                {
                    &*source
                } else {
                    anyhow::bail!("Tried creating {:?} from {:?}", self.target, self.source);
                };

                docker_pull_and_tag(source, s)
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
            TargetArtifact::Docker(s) => docker_push(&*s),
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
) -> Vec<(SourceArtifact, TargetArtifact)> {
    if release.product == Product::Actyx && os_arch.arch == Arch::x86_64 && os_arch.os == OS::linux {
        // Multiarch image, so just do it once
        let mut out = vec![];
        let source = SourceArtifact {
            os_arch,
            release: release.clone(),
            r#type: SourceType::Docker(format!("docker.io/actyx/cosmos:actyx-{}", hash)),
        };

        out.push((
            source.clone(),
            TargetArtifact::Docker(format!("docker.io/actyx/actyx:{}", release.version)),
        ));
        if is_newest {
            out.push((
                source,
                TargetArtifact::Docker("docker.io/actyx/actyx:latest".to_string()),
            ));
        };
        out
    } else {
        vec![]
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
                    file_name: format!("actyx-{}-macos-{}.tar.gz", version, output_arch),
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
                    file_name: format!("actyx-cli-{}-macos-{}.tar.gz", version, output_arch),
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
                //out.push((
                //    "node-manager-linux/actyx-node-manager.rpm".to_string(),
                //    TargetArtifact::Blob {
                //        pre_processing: PreProcessing::None,
                //        file_name: format!("actyx-node-manager-amd64-{}.rpm", version),
                //        local_result: None,
                //    },
                //));
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
    let out = Command::new("az")
        .args(&[
            "storage",
            "blob",
            "exists",
            "--account-name",
            "axartifacts",
            "--container",
            &*container.to_string(),
            "--name",
            name,
        ])
        .output()?;
    let stdout = String::from_utf8(out.stdout)?;
    let stderr = String::from_utf8(out.stderr)?;
    log::trace!("stdout {}", stdout);
    log::trace!("stderr {}", stderr);
    // Always returns 0, even if the blob doesn't exist.
    anyhow::ensure!(out.status.success(), "stdout: {}, stderr: {}", stdout, stderr);
    // Output:
    // {
    //   "exists": false | true
    // }
    Ok(stdout.contains("true"))
}
fn blob_download(name: &str, in_dir: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let file_name = name.split('/').last().unwrap();
    let out_file = in_dir.as_ref().join(file_name);
    let out = Command::new("az")
        .args(&[
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
        ])
        .output()?;
    let stdout = String::from_utf8(out.stdout)?;
    let stderr = String::from_utf8(out.stderr)?;
    log::trace!("stdout {}", stdout);
    log::trace!("stderr {}", stderr);
    anyhow::ensure!(out.status.success(), "stdout: {}, stderr: {}", stdout, stderr);
    Ok(out_file)
}

fn blob_upload(source_file: impl AsRef<Path>, name: &str) -> anyhow::Result<()> {
    let out = Command::new("az")
        .args(&[
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
        ])
        .output()?;

    let stdout = String::from_utf8(out.stdout)?;
    let stderr = String::from_utf8(out.stderr)?;
    log::trace!("stdout {}", stdout);
    log::trace!("stderr {}", stderr);
    anyhow::ensure!(out.status.success(), "stdout: {}, stderr: {}", stdout, stderr);
    Ok(())
}

fn package_tar_gz(source: impl AsRef<Path>, target: impl AsRef<Path>, binary_name: Option<&str>) -> anyhow::Result<()> {
    let output = File::create(target)?;
    let name = binary_name
        .or_else(|| source.as_ref().file_name().map(|x| x.to_str().unwrap()))
        .unwrap();

    let enc = GzEncoder::new(output, Compression::best());
    let mut tar = tar::Builder::new(enc);
    tar.append_path_with_name(&source, &name)?;
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

fn docker_image_exists(image: &str) -> anyhow::Result<bool> {
    let cmd = Command::new("docker").args(&["manifest", "inspect", image]).output()?;
    Ok(cmd.status.success())
}

fn docker_pull_and_tag(source: &str, target: &str) -> anyhow::Result<()> {
    let cmd = Command::new("docker").args(&["pull", source]).output()?;
    anyhow::ensure!(cmd.status.success(), "Error pulling {}", source);
    let cmd = Command::new("docker").args(&["tag", source, target]).output()?;
    anyhow::ensure!(cmd.status.success(), "Error tagging {}", source);
    Ok(())
}
fn docker_push(target: &str) -> anyhow::Result<()> {
    let cmd = Command::new("docker").args(&["push", target]).output()?;
    anyhow::ensure!(cmd.status.success(), "Error pushing {}", target);
    Ok(())
}
