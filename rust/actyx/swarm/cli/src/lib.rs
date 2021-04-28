use actyxos_sdk::{language::Query, Payload, StreamNr, TagSet};
use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;
use swarm::SwarmConfig;
use trees::axtrees::AxKey;

#[derive(Debug, StructOpt)]
pub struct Config {
    #[structopt(long)]
    path: Option<PathBuf>,
    #[structopt(long)]
    node_name: Option<String>,
}

impl From<Config> for SwarmConfig {
    fn from(config: Config) -> Self {
        Self {
            db_path: config.path,
            node_name: config.node_name,
            enable_mdns: true,
            enable_publish: true,
            topic: "swarm-cli".into(),
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
            ..Default::default()
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    Append(StreamNr, Vec<(TagSet, Payload)>),
    Query(Query),
    Exit,
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Append(nr, events) => {
                write!(f, ">append {} {}", nr, serde_json::to_string(events).unwrap())?;
            }
            Self::Query(expr) => write!(f, ">query {}", expr)?,
            Self::Exit => write!(f, ">exit")?,
        }
        Ok(())
    }
}

impl std::str::FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if s.starts_with(">query ") {
            Ok(Self::Query(s.split_at(7).1.parse()?))
        } else if s.starts_with(">append ") {
            let s = s.split_at(8).1;
            let mut iter = s.splitn(2, ' ');
            let nr: u64 = iter.next().unwrap().parse()?;
            let events = serde_json::from_str(iter.next().unwrap())?;
            Ok(Self::Append(nr.into(), events))
        } else if s.starts_with(">exit") {
            Ok(Self::Exit)
        } else {
            Err(anyhow::anyhow!("invalid command '{}'", s))
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Event {
    Result((u64, AxKey, Payload)),
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Result(res) => {
                write!(f, "<result {}", serde_json::to_string(res).unwrap())?;
            }
        }
        Ok(())
    }
}

impl std::str::FromStr for Event {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if s.starts_with("<result ") {
            let s = s.split_at(8).1;
            Ok(Self::Result(serde_json::from_str(s)?))
        } else {
            Err(anyhow::anyhow!("invalid event '{}'", s))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyxos_sdk::tags;

    #[test]
    fn test_command() -> Result<()> {
        let command = &[
            Command::Append(
                42.into(),
                vec![(tags!("a", "b"), Payload::from_json_str("{}").unwrap())],
            ),
            Command::Query("FROM 'a' & 'b' | 'c'".parse().unwrap()),
            Command::Exit,
        ];
        for cmd in command.iter() {
            let cmd2: Command = cmd.to_string().parse()?;
            assert_eq!(cmd, &cmd2);
        }
        Ok(())
    }

    #[test]
    fn test_event() -> Result<()> {
        let event = &[Event::Result((
            0,
            AxKey::new(tags!(), 0, 0),
            Payload::from_json_str("{}").unwrap(),
        ))];
        for ev in event.iter() {
            let ev2: Event = ev.to_string().parse()?;
            assert_eq!(ev, &ev2);
        }
        Ok(())
    }
}
