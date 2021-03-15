use crate::cmd;
use anyhow::Result;
use async_trait::async_trait;
use ax_config::StoreConfig;
use clap::{App, Arg, ArgMatches, SubCommand};
use future::TryFutureExt;
use futures::{future, stream};
use postgres::{Client, Config, NoTls};
use stream::StreamExt;
use swarm::{BanyanStore, Ipfs};
use util::pinned_resource::PinnedResource;

pub struct Cmd;

pub fn args() -> App<'static, 'static> {
    SubCommand::with_name("pubsubToPg")
        .about("Upload data from a pubsub topic to a postgres database")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("verbosity level"),
        )
        .arg(
            Arg::with_name("username")
                .short("U")
                .long("username")
                .value_name("User")
                .help("User name. Will be taken from PGUSER or USER environment variable if not provided")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dbname")
                .short("d")
                .long("dbname")
                .value_name("Database")
                .help("Database name. Will be taken from PGDATABASE environment variable if not provided")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("table")
                .help("table name - defaults to topic name")
                .long("table")
                .value_name("Table")
                .help("Table name")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("column")
                .help("column name")
                .long("column")
                .value_name("Column")
                .help("Column name")
                .takes_value(true)
                .default_value("data"),
        )
        .arg(
            Arg::with_name("host")
                .help("host of Postgres database")
                .long("host")
                .short("h")
                .default_value("localhost")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .help("port of Postgres database")
                .long("port")
                .short("p")
                .default_value("5432")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("delete_older_than")
                .help("delete items older than a pg interal string, e.g. 7days")
                .long("delete_older_than")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("topic")
                .short("t")
                .long("topic")
                .value_name("Topic")
                .takes_value(true),
        )
}

fn make_create_statement(table: &str, column: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {} (
    id serial NOT NULL PRIMARY KEY,
    time TIMESTAMP DEFAULT NOW(),
    {} jsonb
);",
        table, column
    )
}

fn make_delete_statement(table: &str, interval: &str) -> String {
    format!("DELETE FROM {} WHERE time < NOW() - INTERVAL '{}';", table, interval)
}

fn make_insert_statement(table: &str, column: &str) -> String {
    format!("INSERT INTO {} ({}) Values ($1);", table, column)
}

const DELETE_INTERVAL_S: u64 = 3600;

async fn upload(
    conn: PinnedResource<Client>,
    client: Ipfs,
    topic: String,
    insert_statement: String,
    delete_statement: Option<String>,
    verbosity: u64,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut msgs = client.subscribe(&topic)?;
    let mut last_delete = std::time::Instant::now();
    while let Some(event) = msgs.next().await {
        match util::serde_util::from_json_or_cbor_slice::<serde_value::Value>(event.as_slice()) {
            Ok(serde_value) => {
                let json_value = serde_json::to_value(&serde_value).map_err(|err| format!("{}", err))?;
                if verbosity > 1 {
                    println!("{}", json_value);
                }
                // it is not possible to create an index with IF NOT EXISTS in older pg.
                // so we don't have an index on time and can't afford to run the delete every time.
                // but running it every hour should be fine. If it is too slow you can always manually
                // add an index.
                let now = std::time::Instant::now();
                let delete_statement = if now.duration_since(last_delete).as_secs() > DELETE_INTERVAL_S {
                    last_delete = now;
                    delete_statement.clone()
                } else {
                    None
                };
                let insert_statement = insert_statement.clone();
                conn.spawn_mut(move |conn| {
                    if let Some(delete_statement) = delete_statement {
                        if verbosity > 0 {
                            println!("Deleting old values! {}", delete_statement);
                        }
                        conn.execute(delete_statement.as_str(), &[])?;
                    }
                    conn.execute(insert_statement.as_str(), &[&json_value])
                })
                .await??;
            }
            Err(cause) => eprintln!("Message is neither CBOR nor JSON {}", cause),
        }
    }
    Ok(())
}

#[async_trait]
impl cmd::Command for Cmd {
    fn name(&self) -> &str {
        "pubsubToPg"
    }

    async fn run(&self, matches: &ArgMatches<'_>, _config: StoreConfig, store: BanyanStore) -> Result<()> {
        // https://www.postgresql.org/docs/9.3/libpq-envars.html
        let client = store.ipfs();
        let default_user = std::env::var("PGUSER").or_else(|_| std::env::var("USER")).ok();
        let default_dbname = std::env::var("PGDATABASE").ok();
        let password = std::env::var("PGPASSWORD").ok();
        let topic = matches.value_of("topic").expect("topic name not provided!").to_string();
        let host = matches.value_of("host").expect("host not provided!").to_string();
        let delete_older_than = matches.value_of("delete_older_than").map(|x| x.to_string());
        let verbosity = matches.occurrences_of("verbose");
        let port = matches
            .value_of("port")
            .map(|port| port.parse::<u16>().expect("not a valid port number"))
            .expect("port not provided!");
        let default_table_name = topic.replace("/", "_").replace("-", "_").to_ascii_lowercase();
        let user = matches
            .value_of("username")
            .or_else(|| default_user.as_deref())
            .expect("user not provided");
        let table = matches
            .value_of("table")
            .map(|x| x.to_string())
            .unwrap_or(default_table_name);
        let column = matches
            .value_of("column")
            .map(|x| x.to_string())
            .expect("column was not provided");
        let db_name = matches
            .value_of("dbname")
            .or_else(|| default_dbname.as_deref())
            .expect("db name not provided");
        let mut config = Config::new();
        config.user(user);
        if let Some(password) = password {
            config.password(password.as_bytes());
        }
        config.dbname(db_name);
        config.port(port);
        config.host(&host);

        let create_statement = make_create_statement(&table, &column);
        let insert_statement = make_insert_statement(&table, &column);
        let delete_statement = delete_older_than.map(|interval| make_delete_statement(&table, &interval));
        if verbosity > 1 {
            println!("SQL statements:");
            println!("{}", create_statement);
            println!("{}", insert_statement);
            if let Some(delete_statement) = delete_statement.as_ref() {
                println!("{}", delete_statement);
            }
            println!();
        }
        let mut conn = config.connect(NoTls).expect("Unable to connect to the database");
        conn.execute(create_statement.as_str(), &[])
            .expect("Unable to execute create statement");
        // execute the delete statement at startup even if we don't get values
        for delete_statement in delete_statement.iter() {
            if verbosity > 0 {
                println!("Deleting old values! {}", delete_statement);
            }
            conn.execute(delete_statement.as_str(), &[])
                .expect("Unable to execute delete statement");
        }
        let conn = PinnedResource::new(|| conn);
        let completion = upload(
            conn,
            client.clone(),
            topic,
            insert_statement,
            delete_statement,
            verbosity,
        );

        completion.map_err(|cause| format!("{}", cause)).await.unwrap();
        Ok(())
    }
}
