use std::str::FromStr;

use anyhow::{bail, Context};
use certs::{AppDomain, DeveloperCertificate, DeveloperCertificateInput};
use crypto::{PrivateKey, PublicKey};
use structopt::StructOpt;
use util::version::NodeVersion;

#[derive(StructOpt, Debug)]
struct CreateOpts {
    #[structopt(long, env, hide_env_values = true)]
    /// Actyx private key
    actyx_private_key: String,
    #[structopt(long)]
    /// Developer's public key
    dev_public_key: String,
    /// Certificate's allowed app domains
    #[structopt(long, required = true)]
    app_domains: Vec<String>,
}

#[derive(StructOpt, Debug)]
enum Commands {
    /// Creates developer certificate
    Create(CreateOpts),
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "Actyx developer certificate utility",
    about = "Manages Actyx developer certificates",
    rename_all = "kebab-case"
)]
struct Opts {
    #[structopt(subcommand)]
    commands: Option<Commands>,
    #[structopt(long)]
    version: bool,
}

fn create_dev_cert(opts: CreateOpts) -> anyhow::Result<()> {
    let ax_private_key = PrivateKey::from_str(&opts.actyx_private_key).context("Unable to parse actyx private key")?;
    let dev_public_key = PublicKey::from_str(&opts.dev_public_key).context("Unable to parse developer's public key")?;
    let mut app_domains: Vec<AppDomain> = Vec::new();
    for x in opts.app_domains {
        match x.parse() {
            Ok(app_domain) => app_domains.push(app_domain),
            Err(x) => bail!("Failed to parse app domain. {}", x),
        }
    }

    let input = DeveloperCertificateInput::new(dev_public_key, app_domains);
    let dev_cert = DeveloperCertificate::new(input, ax_private_key)?;
    let serialized = serde_json::to_string(&dev_cert).unwrap();
    println!("{}", serialized);

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::from_args();

    match opts {
        Opts { version: true, .. } => {
            let app = Opts::clap();
            let mut buf = Vec::new();
            app.write_version(&mut buf).unwrap();
            let bin_version = std::str::from_utf8(buf.as_slice()).unwrap().to_string();
            println!("{} {}", bin_version, NodeVersion::get());
            Ok(())
        }
        Opts {
            commands: Some(cmd), ..
        } => match cmd {
            Commands::Create(opts) => create_dev_cert(opts),
        },
        _ => {
            let mut app = Opts::clap();
            app.write_long_help(&mut std::io::stderr()).unwrap();
            println!();
            Ok(())
        }
    }
}
