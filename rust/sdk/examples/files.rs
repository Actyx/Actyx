use std::{
    io::Write,
    path::{Path, PathBuf},
};

use actyx_sdk::{app_id, service::DirectoryChild, AppManifest, HttpClient};
use asynchronous_codec::{BytesCodec, Framed};
use futures::{
    future::{try_join_all, BoxFuture},
    FutureExt,
};
use reqwest::{multipart::Part, Body};
use structopt::StructOpt;
use tokio::{fs::File, io::AsyncWriteExt};
use tokio_util::compat::*;
use url::Url;

async fn mk_http_client() -> anyhow::Result<HttpClient> {
    let app_manifest = AppManifest::new(
        app_id!("com.example.actyx-offsets"),
        "Offsets Example".into(),
        "0.1.0".into(),
        None,
    );
    let url = Url::parse("http://localhost:4454").unwrap();
    HttpClient::new(url, app_manifest).await
}

#[derive(StructOpt)]
struct Opt {
    #[structopt(flatten)]
    command: Command,
}
#[derive(StructOpt)]
enum Command {
    /// Add files or directories recursively. Returns the hash identifying the content.
    Add {
        /// Path to a local file or a directory to add
        file: PathBuf,
    },
    /// List files or directories recursively. Note that this will also fetch all the files'
    /// bytes, so the current implementation is merely for demo purposes.
    Ls {
        /// Name or a Cid, and an optional path
        /// Examples: bafybeibogm7ogaite4rjjnw6laiszuxw5hvwkp2cj726rr7zup3yw34tea,
        /// <cid>/types
        name_or_cid: String,
    },
    Get {
        /// Name or a Cid, and an optional path
        /// Examples: bafybeibogm7ogaite4rjjnw6laiszuxw5hvwkp2cj726rr7zup3yw34tea,
        /// <cid>/types
        name_or_cid: String,
        /// Output path
        #[structopt(short, long)]
        output: PathBuf,
    },
    // Display the data
    Cat {
        /// Name or a Cid, and an optional path
        /// Examples: bafybeibogm7ogaite4rjjnw6laiszuxw5hvwkp2cj726rr7zup3yw34tea,
        /// <cid>/types
        name_or_cid: String,
    },
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    let service = mk_http_client().await?;
    match opt.command {
        Command::Add { file } => {
            let files: Box<dyn Iterator<Item = Part>> = if file.is_file() {
                Box::new(std::iter::once(mk_part(file).await?))
            } else {
                Box::new(add_dir(file, "".into()).await?.into_iter())
            };

            let cid = service.files_post(files).await?;
            println!("{}", cid);
        }
        Command::Ls { name_or_cid } => {
            list_file_or_dir(&service, name_or_cid, 0).await?;
        }
        Command::Get { name_or_cid, output } => {
            get_file_or_dir(service, name_or_cid, output.clone()).await?;
            println!("Please find your output in {}", output.display());
        }
        Command::Cat { name_or_cid } => match service.files_get(&name_or_cid).await? {
            actyx_sdk::service::FilesGetResponse::File { bytes, .. } => {
                std::io::stdout().lock().write_all(&bytes[..])?;
            }
            actyx_sdk::service::FilesGetResponse::Directory { .. } => {
                anyhow::bail!("{} is a directory", name_or_cid);
            }
        },
    }

    Ok(())
}

fn list_file_or_dir(client: &HttpClient, name_or_cid: String, level: usize) -> BoxFuture<'_, anyhow::Result<()>> {
    async move {
        let response = client.files_get(&name_or_cid).await?;
        match response {
            actyx_sdk::service::FilesGetResponse::File { name, bytes, mime } if level == 0 => {
                println!("{} ({}): {}", name, mime, bytes.len());
            }
            actyx_sdk::service::FilesGetResponse::Directory { name, cid, children } => {
                let indent = level * 4;
                if indent == 0 {
                    println!("{:<34}{:<10}{:<10}", name, 0, cid);
                }
                for DirectoryChild { cid, name, size } in children {
                    println!("{:indent$}├── {:<30}{:<10}{:<10}", "", name, size, cid, indent = indent);
                    list_file_or_dir(client, cid.to_string(), level + 1).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    .boxed()
}

fn get_file_or_dir(
    client: HttpClient,
    name_or_cid: String,
    write_to: PathBuf,
) -> BoxFuture<'static, anyhow::Result<()>> {
    async move {
        match client.files_get(&name_or_cid).await? {
            actyx_sdk::service::FilesGetResponse::File { bytes, .. } => {
                let mut file = File::create(write_to).await?;
                file.write_all(&bytes[..]).await?;
            }
            actyx_sdk::service::FilesGetResponse::Directory { children, .. } => {
                std::fs::create_dir_all(&write_to)?;
                let futs = children.into_iter().map(|DirectoryChild { cid, name, .. }| {
                    get_file_or_dir(client.clone(), cid.to_string(), write_to.join(name))
                });
                try_join_all(futs).await?;
            }
        }
        Ok(())
    }
    .boxed()
}

fn add_dir(dir: PathBuf, rel_path: String) -> BoxFuture<'static, anyhow::Result<impl IntoIterator<Item = Part>>> {
    async move {
        let mut buf = vec![];
        for entry in dir.read_dir()? {
            let entry = entry?;
            if entry.metadata()?.is_dir() {
                for c in add_dir(
                    entry.path(),
                    format!("{}{}/", rel_path, entry.file_name().to_string_lossy()),
                )
                .await?
                .into_iter()
                {
                    buf.push(c);
                }
            } else {
                buf.push(mk_part(entry.path()).await?.file_name(format!(
                    "{}{}",
                    rel_path,
                    entry.file_name().to_string_lossy()
                )));
            }
        }
        Ok(buf)
    }
    .boxed()
}

async fn mk_part(file: impl AsRef<Path>) -> anyhow::Result<Part> {
    let f = file.as_ref();
    anyhow::ensure!(f.is_file(), "{} is not a file!", f.display());
    let stream = Framed::new(File::open(&f).await?.compat(), BytesCodec);
    Ok(
        Part::stream_with_length(Body::wrap_stream(stream), f.metadata()?.len()).file_name(
            f.file_name()
                .expect("File must have a name")
                .to_string_lossy()
                .to_string(),
        ),
    )
}
