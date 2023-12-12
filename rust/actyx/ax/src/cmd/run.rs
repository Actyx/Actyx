use anyhow::Result;
use ax_core::{
    node::{BindTo, PortOrHostPort},
    util::SocketAddrHelper,
};
use std::{
    convert::TryInto,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    str::FromStr,
};

#[derive(Debug, Clone)]
pub enum Color {
    Off,
    Auto,
    On,
}

impl FromStr for Color {
    type Err = NoColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "1" | "on" | "true" => Ok(Self::On),
            "0" | "off" | "false" => Ok(Self::Off),
            "auto" => Ok(Self::Auto),
            _ => Err(NoColor),
        }
    }
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display(fmt = "allowed values are 1, on, true, 0, off, false, auto (case insensitive)")]
pub struct NoColor;

#[derive(clap::Parser, Debug, Clone)]
#[command(
    name = "ax",
    about = "run the ax distributed event database",
    after_help = "For one-off log verbosity override, you may start with the environment variable \
        RUST_LOG set to “debug” or “node=debug,info” (the former logs all debug messages while \
        the latter logs at debug level for the “node” code module and info level for everything \
        else).
        ",
    rename_all = "kebab-case"
)]
pub struct RunOpts {
    /// Path where to store all the data of the Actyx node.
    #[arg(
        long,
        env = "ACTYX_PATH",
        long_help = "Path where to store all the data of the Actyx node. \
            Defaults to creating <current working dir>/actyx-data"
    )]
    pub working_dir: Option<PathBuf>,

    #[command(flatten)]
    pub bind_options: BindToOpts,

    #[arg(short, long, hide = true)]
    pub random: bool,

    /// Control whether to use ANSI color sequences in log output.
    #[arg(
        long,
        env = "ACTYX_COLOR",
        long_help = "Control whether to use ANSI color sequences in log output. \
            Valid values (case insensitive) are 1, true, on, 0, false, off, auto \
            (default is on, auto only uses colour when stderr is a terminal). \
            Defaults to 1."
    )]
    pub log_color: Option<Color>,

    /// Output logs as JSON objects (one per line)
    #[arg(
        long,
        env = "ACTYX_LOG_JSON",
        long_help = "Output logs as JSON objects (one per line) if the value is \
            1, true, on or if stderr is not a terminal and the value is auto \
            (all case insensitive). Defaults to 0."
    )]
    pub log_json: Option<Color>,
}

#[derive(clap::Parser, Clone, Debug)]
pub struct BindToOpts {
    /// Port to bind to for management connections.
    #[arg(
        long,
        default_value = "4458",
        long_help = "Port to bind to for management connections. Specifying a single number is \
            equivalent to “0.0.0.0:<port> [::]:<port>”, thus specifying 0 usually selects \
            different ports for IPv4 and IPv6. Specify 0.0.0.0:<port> to only use IPv4, or \
            [::]:<port> for only IPv6; you may also specify other names or addresses or leave off \
            the port number."
    )]
    bind_admin: Vec<PortOrHostPort<4458>>,

    /// Port to bind to for intra swarm connections.
    #[arg(
        long,
        default_value = "4001",
        long_help = "Port to bind to for intra swarm connections. \
            The same rules apply as for the admin port."
    )]
    bind_swarm: Vec<PortOrHostPort<4001>>,

    /// Port to bind to for the API used by apps.
    #[arg(
        long,
        default_value = "localhost",
        long_help = "Port to bind to for the API used by apps. \
            The same rules apply as for the admin port, except that giving only a port binds \
            to 127.0.0.1 only. The default port is 4454."
    )]
    bind_api: Vec<PortOrHostPort<4454>>,
}

impl TryInto<BindTo> for BindToOpts {
    type Error = anyhow::Error;
    fn try_into(self) -> anyhow::Result<BindTo> {
        let api = fold(
            |port| SocketAddrHelper::from_ip_port(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            self.bind_api,
        )?;
        let admin = fold(SocketAddrHelper::unspecified, self.bind_admin)?;
        let swarm = fold(SocketAddrHelper::unspecified, self.bind_swarm)?;
        Ok(BindTo { admin, swarm, api })
    }
}

fn fold<const N: u16>(
    port: impl FnOnce(u16) -> anyhow::Result<SocketAddrHelper>,
    input: Vec<PortOrHostPort<N>>,
) -> anyhow::Result<SocketAddrHelper> {
    if input.is_empty() {
        anyhow::bail!("no value provided");
    }
    let mut found_port = None;
    let mut host_port: Option<SocketAddrHelper> = None;
    for i in input.into_iter() {
        match i {
            PortOrHostPort::Port(p) => {
                if found_port.is_some() {
                    anyhow::bail!("Multiple single port directives not supported");
                } else if host_port.is_some() {
                    anyhow::bail!("Both port directive and host:port combination not supported");
                } else {
                    found_port.replace(p);
                }
            }
            PortOrHostPort::HostPort(addr) => {
                if found_port.is_some() {
                    anyhow::bail!("Both port directive and host:port combination not supported");
                } else if let Some(x) = host_port.as_mut() {
                    x.append(addr);
                } else {
                    let _ = host_port.replace(addr);
                }
            }
        }
    }
    found_port
        .map(port)
        .or_else(|| host_port.map(Ok))
        .expect("Input must not be empty")
}
