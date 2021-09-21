use actyx_sdk::{legacy::SourceId, NodeId, StreamId};
use anyhow::Context;
use axlib::{
    node_connection::{to_node_id, NodeConnection},
    private_key::AxPrivateKey,
};
use crossterm::event::{Event, KeyCode, KeyEvent as Key, MouseEventKind};
use futures::{pin_mut, stream::FuturesUnordered, FutureExt, Stream, StreamExt, TryFutureExt};
use multiaddr::{Multiaddr, Protocol};
use node::migration::v1::{self, assert_v1, convert_swarm_key_v1_v2};
use parking_lot::Mutex;
use settings::Repository;
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryInto,
    fs,
    io::{self, Write},
    path::Path,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use swarm::{
    convert::{info_from_v1_index_store, V1MigrationEvent},
    event_store::EventStore,
    BanyanStore, SwarmConfig,
};
use tokio::{select, sync::mpsc::Receiver, time::timeout};
use trees::query::{LamportQuery, TagExprQuery, TimeQuery};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
    Terminal,
};
use unicode_width::UnicodeWidthStr;
use util::formats::events_protocol::{EventsRequest, EventsResponse};

pub enum DisseminationState {
    Waiting(NodeConnection),
    Unreachable { error: String },
    UpToDate,
}

fn get_connected_peers(store: &BanyanStore) -> BTreeSet<(NodeId, Multiaddr)> {
    store
        .ipfs()
        .connections()
        .into_iter()
        .map(|(peer_id, mut multiaddr)| {
            multiaddr.pop(); // peer id
            multiaddr.pop(); // tcp
            multiaddr.push(Protocol::Tcp(4458));
            multiaddr.push(Protocol::P2p(peer_id.into()));

            let node_id = to_node_id(peer_id);
            (node_id, multiaddr)
        })
        .collect()
}

pub async fn assess_v2_swarm(
    store: BanyanStore,
    v1_sources: BTreeSet<SourceId>,
    out: impl Write + Send,
    key_rx: &mut Receiver<anyhow::Result<Event>>,
) -> anyhow::Result<()> {
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    let priv_key = get_private_key(&mut terminal, key_rx).await?;
    tracing::info!("got private key {:?}", priv_key);

    let source_mapping: Arc<Mutex<BTreeMap<SourceId, Option<V1MigrationEvent>>>> = Arc::new(Mutex::new(
        v1_sources.into_iter().map(|s| (s, None)).collect::<BTreeMap<_, _>>(),
    ));
    let migration_events = migration_events(&store);
    let mapping = Arc::downgrade(&source_mapping);
    tokio::spawn(async move {
        pin_mut!(migration_events);
        while let Some(x) = migration_events.next().await {
            if let Some(map) = mapping.upgrade() {
                map.lock().insert(x.v1_source_id, Some(x));
            } else {
                break;
            }
        }
    });

    let event_store = EventStore::new(store.clone());
    let mut state: BTreeMap<_, _> = get_connected_peers(&store)
        .into_iter()
        .map(|(node_id, addr)| {
            (
                node_id,
                DisseminationState::Waiting(NodeConnection::from_str(&*addr.to_string()).unwrap()),
            )
        })
        .collect();

    terminal.clear()?;
    draw_peer_list(&mut terminal, &mut state)?;
    while state.iter().any(|(_, s)| matches!(s, DisseminationState::Waiting(_))) {
        // check for new peers
        for (node, addr) in get_connected_peers(&store) {
            state
                .entry(node)
                .or_insert_with(|| DisseminationState::Waiting(NodeConnection::from_str(&*addr.to_string()).unwrap()));
        }

        // construct parallel requests
        let mut streams = state
            .iter_mut()
            .filter_map(|(n, s)| {
                if let DisseminationState::Waiting(a) = s {
                    Some((n, a))
                } else {
                    None
                }
            })
            .map(|(n, a)| {
                timeout(
                    Duration::from_millis(500),
                    a.request_events(&priv_key, EventsRequest::Offsets)
                        // it's not a stream, but only a single response ..
                        .map_ok(move |s| s.into_future().map(|(r, _)| r))
                        .map_err(move |err| (*n, anyhow::Error::from(err)))
                        .and_then(move |fut| fut.map(move |r| (*n, r)).map(Ok)),
                )
                .map_err(move |e| (*n, anyhow::Error::from(e)))
            })
            .collect::<FuturesUnordered<_>>();

        // evaluate nodes responses
        let mut state_updates = BTreeMap::new();
        while let Some(x) = streams.next().await {
            match x {
                Ok(Ok((node_id, resp))) => {
                    if let Some(EventsResponse::Offsets(offsets)) = resp {
                        let local_offsets = event_store.current_offsets();
                        if offsets.present >= local_offsets.present() {
                            tracing::info!("Node {} is up to date.", node_id);
                            state_updates.insert(node_id, DisseminationState::UpToDate);
                        }
                    }
                }
                Ok(Err((node_id, e))) | Err((node_id, e)) => {
                    tracing::error!("Node {} is not reachable {:?}", node_id, e);
                    state_updates.insert(node_id, DisseminationState::Unreachable { error: e.to_string() });
                }
            }
        }
        drop(streams);
        // update state map with nodes responses
        state.append(&mut state_updates);

        draw_peer_list(&mut terminal, &mut state)?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tracing::info!("All connected peers up to date!");
    drain_channel(key_rx);
    while !matches!(key_rx.recv().await, Some(Ok(Event::Key(_)))) {}

    terminal.clear()?;

    loop {
        match timeout(Duration::from_millis(100), key_rx.recv()).await {
            Ok(Some(Ok(Event::Key(Key {
                code: KeyCode::Char('q'),
                ..
            }))))
            | Ok(None) => {
                break;
            }
            _ => {}
        }

        let json = serde_json::to_string_pretty(
            &source_mapping
                .lock()
                .iter()
                .filter_map(|(source, maybe_ev)| maybe_ev.as_ref().map(|e| (*source, e.v2_stream_id)))
                .collect::<BTreeMap<_, _>>(),
        )?;
        draw_summary(&mut terminal, &source_mapping.lock(), json)?;
    }

    terminal.clear()?;

    tracing::info!("Goodbye!");

    Ok(())
}

fn draw_summary(
    terminal: &mut Terminal<impl Backend>,
    state: &BTreeMap<SourceId, Option<V1MigrationEvent>>,
    json: String,
) -> anyhow::Result<()> {
    terminal.draw(|f| {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Min(0)].as_ref())
            .margin(5)
            .split(f.size());

        f.render_widget(Paragraph::new("Migration summary. Press 'q' to exit."), areas[0]);
        let migrated_sources = state
            .iter()
            .filter(|(_, x)| x.is_some())
            .map(|(s, _)| *s)
            .collect::<BTreeSet<_>>();
        if migrated_sources.len() < state.len() {
            f.render_widget(
                Paragraph::new(format!(
                    "Only {} of {} sources were migrated. Please find the mapping below:",
                    migrated_sources.len(),
                    state.len()
                )),
                areas[1],
            )
        } else {
            f.render_widget(
                Paragraph::new(format!(
                    "Migrated all {} sources, please find the mapping below:",
                    state.len()
                )),
                areas[1],
            );
        }
        f.render_widget(Paragraph::new(json), areas[2]);
    })?;
    Ok(())
}

fn draw_peer_list(
    terminal: &mut Terminal<impl Backend>,
    state: &mut BTreeMap<NodeId, DisseminationState>,
) -> anyhow::Result<()> {
    terminal.draw(|f| {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Percentage(90),
                    Constraint::Percentage(5),
                ]
                .as_ref(),
            )
            .margin(5)
            .split(f.size());
        let table = {
            let rows = state.iter().enumerate().map(|(idx, (k, v))| {
                let mut c = vec![idx.to_string(), k.to_string()];
                let status = match v {
                    DisseminationState::Waiting(_) => "Waiting ... ".to_string(),
                    DisseminationState::Unreachable { error } => format!("❌ ({})", error),
                    DisseminationState::UpToDate => "✔️ (and private key set up)".to_string(),
                };
                c.push(status);

                let cells = c.into_iter().map(Cell::from);
                Row::new(cells)
            });

            let header_cells = ["", "Node ID", "State"]
                .iter()
                .map(|h| Cell::from(*h).style(Style::default().fg(Color::Blue)));
            let table_header = Row::new(header_cells).height(1).bottom_margin(1);
            Table::new(rows)
                .header(table_header)
                .widths(&[
                    Constraint::Percentage(3),
                    Constraint::Percentage(37),
                    Constraint::Percentage(60),
                ])
                .block(Block::default().title("V2 nodes").borders(Borders::ALL))
        };
        f.render_widget(
            Paragraph::new("Waiting for update to disseminate in the swarm .."),
            areas[0],
        );
        f.render_widget(table, areas[1]);
        let is_finished = !state.iter().any(|(_, s)| matches!(s, DisseminationState::Waiting(_)));
        let with_errors = state
            .iter()
            .any(|(_, s)| matches!(s, DisseminationState::Unreachable { .. }));
        if is_finished {
            if with_errors {
                f.render_widget(
                    Paragraph::new("Migration finished! There have been errors. Press any key to continue.. "),
                    areas[0],
                );
            } else {
                f.render_widget(
                    Paragraph::new("Migration successful! Press any key to continue.. "),
                    areas[0],
                );
            }
        }
    })?;
    Ok(())
}

fn drain_channel(rx: &mut Receiver<anyhow::Result<Event>>) {
    let waker = futures::task::noop_waker();
    let mut context = futures::task::Context::from_waker(&waker);
    while rx.poll_recv(&mut context).is_ready() {}
}

#[allow(clippy::clippy::future_not_send)]
async fn get_private_key(
    terminal: &mut Terminal<impl Backend>,
    key_rx: &mut Receiver<anyhow::Result<Event>>,
) -> anyhow::Result<AxPrivateKey> {
    let mut key = String::new();

    drain_channel(key_rx);
    terminal.clear()?;
    loop {
        terminal.draw(|f| {
            let area = centered_rect(60, 40, f.size());
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                .split(area);
            let block = Paragraph::new(key.as_ref())
                .block(
                    Block::default()
                        .title("Please insert your private key")
                        .borders(Borders::ALL),
                )
                .style(Style::default().fg(Color::White).bg(Color::Black));
            f.render_widget(block, chunks[0]);
            if !key.is_empty() {
                f.render_widget(Paragraph::new("Not a valid private key"), chunks[1]);
            }
            f.set_cursor(area.x + key.width() as u16 + 1, area.y + 1);
        })?;
        if let Some(x) = key_rx.recv().await {
            if let Event::Key(k) = x? {
                match k.code {
                    KeyCode::Char(c) if c == '\n' => {}
                    KeyCode::Char(c) => key.push(c),
                    KeyCode::Backspace => {
                        key.pop();
                    }
                    _ => anyhow::bail!("Aborted"),
                }
            }
        }
        match key.parse() {
            Ok(x) => return Ok(x),
            Err(e) => tracing::debug!("Not a valid key {:?}", e),
        }
    }
}
pub struct MigratedSwarm {
    pub store: BanyanStore,
}
pub async fn v1_migrate_sources_and_disseminate(
    working_dir: impl AsRef<Path> + Send,
    sources: BTreeSet<SourceId>,
) -> anyhow::Result<MigratedSwarm> {
    let v1_dir = assert_v1(&working_dir)
        .with_context(|| format!("Error parsing {} as v1 directory", working_dir.as_ref().display()))?;

    let Settings {
        bootstrap: initial_peers,
        swarm_key,
        topic,
    } = get_settings(&v1_dir.settings_repo)?;

    let migration_working_dir = tempfile::tempdir()?;
    copy_dir_recursive(&working_dir, migration_working_dir.path())?;

    drop(v1_dir);
    v1::migrate(
        migration_working_dir.path(),
        migration_working_dir.path(),
        sources,
        false,
        false,
    )
    .map_err(|e| {
        tracing::error!(target: "MIGRATION", "Error during migration: {:?}", e);
        e
    })?;

    let db_name = topic.replace("/", "_");
    let db_path = migration_working_dir
        .path()
        .join("store")
        .join(&*format!("{}.sqlite", db_name));
    let index_store = Some(Arc::new(Mutex::new(rusqlite::Connection::open(
        migration_working_dir.path().join("node.sqlite"),
    )?)));
    tracing::debug!(
        "v1_migrate_sources_and_disseminate: Creating temporary store at path {:?}, index_store {:?}, migration_working_dir: {:?}",
        db_path,
        index_store, migration_working_dir.path()
    );
    let store = start_ephemeral_readonly_swarm(swarm_key, initial_peers, topic, db_path, index_store).await?;

    Ok(MigratedSwarm { store })
}
struct Settings {
    bootstrap: Vec<Multiaddr>,
    swarm_key: String,
    topic: String,
}
fn get_settings(repo: &Repository) -> anyhow::Result<Settings> {
    let topic: String = repo
        .get_settings(&"com.actyx.os/services/eventService/topic".parse().unwrap(), false)?
        .as_str()
        .unwrap()
        .into();

    let bootstrap = {
        let mut val = repo.get_settings(&"com.actyx.os/general/bootstrapNodes".parse().unwrap(), false)?;
        for v in val.as_array_mut().unwrap() {
            *v = serde_json::Value::String(v.as_str().unwrap().replace("/ipfs/", "/p2p/"));
        }
        serde_json::from_value(val)?
    };
    let v1_swarm_key = repo.get_settings(&"com.actyx.os/general/swarmKey".parse().unwrap(), false)?;
    let swarm_key = convert_swarm_key_v1_v2(v1_swarm_key.as_str().unwrap())?;

    Ok(Settings {
        bootstrap,
        swarm_key,
        topic,
    })
}
pub struct MixedSwarmOverview {
    pub v1_sources: BTreeSet<SourceId>,
    pub to_migrate: BTreeSet<SourceId>,
}
pub async fn v1_overview(
    working_dir: impl AsRef<Path> + Send,
    out: impl Write + Send,
    key_rx: &mut Receiver<anyhow::Result<Event>>,
) -> anyhow::Result<MixedSwarmOverview> {
    let v1_dir = assert_v1(&working_dir)
        .with_context(|| format!("Error parsing {} as v1 directory", working_dir.as_ref().display()))?;

    let Settings {
        bootstrap: initial_peers,
        swarm_key,
        topic,
    } = get_settings(&v1_dir.settings_repo)?;
    tracing::info!(
        "Found v1 data; bootstrap_nodes {:?}, topic {}, swarm_key {}",
        initial_peers,
        topic,
        vec!['*'; swarm_key.len()].into_iter().collect::<String>(),
    );
    let temp = tempfile::tempdir()?;

    tracing::debug!("v1_overview: Creating temporary store at path {:?}", temp.path());
    let store = start_ephemeral_readonly_swarm(swarm_key, initial_peers, topic, temp.path().join("db"), None).await?;
    tracing::debug!("reading info from existing v1 index store at {:?}", &v1_dir.index_db);
    let info = info_from_v1_index_store(&v1_dir.index_db).context("getting v1 db info")?;
    let v1_source_ids: Vec<SourceId> = info.roots.keys().copied().collect();
    tracing::info!("found v1 source ids: {:?}", v1_source_ids);

    let to_migrate = get_sources_to_migrate(v1_source_ids.into_iter(), store, out, key_rx).await?;

    Ok(MixedSwarmOverview {
        to_migrate,
        v1_sources: info.roots.iter().map(|(s, _)| *s).collect(),
    })
}

async fn start_ephemeral_readonly_swarm(
    swarm_key: String,
    initial_peers: Vec<Multiaddr>,
    topic: String,
    blocks_db: impl AsRef<Path> + Send,
    index_store: Option<Arc<Mutex<rusqlite::Connection>>>,
) -> anyhow::Result<BanyanStore> {
    let psk: [u8; 32] = base64::decode(&swarm_key)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid swarm key"))?;
    let cfg = SwarmConfig {
        bootstrap_addresses: initial_peers,
        psk: Some(psk),
        enable_metrics: false,
        enable_discovery: false,
        enable_root_map: true,
        enable_fast_path: true,
        enable_slow_path: true,
        enable_mdns: true,
        topic,
        db_path: Some(blocks_db.as_ref().into()),
        index_store,
        listen_addresses: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap(), "/ip6/::/tcp/0".parse().unwrap()],
        ..SwarmConfig::basic()
    };
    tracing::debug!("Creating BanyanStore with config {:?}", cfg);
    BanyanStore::new(cfg).await
}

fn migration_events(store: &BanyanStore) -> impl Stream<Item = V1MigrationEvent> {
    store
        .stream_filtered_stream_ordered(TagExprQuery::new(
            std::iter::once(trees::stags!("migration")),
            LamportQuery::all(),
            TimeQuery::all(),
        ))
        .filter_map(|x| async move { x.ok() })
        .filter_map(|(_, _, payload)| async move { payload.extract::<V1MigrationEvent>().ok() })
}

async fn get_sources_to_migrate(
    v1_source_ids: impl Iterator<Item = SourceId> + Send,
    store: BanyanStore,
    out: impl Write + Send,
    key_rx: &mut Receiver<anyhow::Result<Event>>,
) -> anyhow::Result<BTreeSet<SourceId>> {
    let migration_events = migration_events(&store);
    pin_mut!(migration_events);

    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    let mut state = State::new(v1_source_ids.collect());
    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut stream_known_streams = store.stream_known_streams();
    // Clear terminal and initial render
    drain_channel(key_rx);
    terminal.clear()?;
    update_view(&mut terminal, &mut state)?;
    let ret = 'outer: loop {
        select! {
            _ = ticker.tick() => {
                state.connected_peers = store.ipfs().peers().len();
            }
            Some(x) = migration_events.next() => {
                state.swarm_migration_state.insert(x.v1_source_id, MigrationState::Found(x.v2_stream_id));
            }
            Some(x) = stream_known_streams.next() => {
                state.known_streams.insert(x);
            }
            Some(ev) = key_rx.recv() => {
                let ev = ev?;
                tracing::debug!("Got terminal ev {:?}", ev);
                match ev {
                    Event::Key(k) => match k.code {
                        KeyCode::Char('q') => {
                            if state.popup.take().is_none() {
                                break 'outer BTreeSet::new();
                            }
                        }
                        KeyCode::Down => state.next(),
                        KeyCode::Up => state.previous(),
                        KeyCode::Home | KeyCode::PageUp => state.first(),
                        KeyCode::End | KeyCode::PageDown => state.last(),
                        KeyCode::Esc => { state.popup = None; },
                        KeyCode::Char('m') => {
                            match &state.popup {
                                Some(Popup::ConfirmSources { sources }) => {
                                    break 'outer sources.clone();
                                }
                                _ => state.confirm_sources(),
                            }
                        }
                        KeyCode::Char('h') => { state.popup = Some(Popup::Help) },
                        KeyCode::Char('\n') | KeyCode::Char(' ') => {
                            if let Some(i) = state.table.selected() {
                                let source = state.swarm_migration_state.keys().copied().nth(i).unwrap();
                                if let Some(MigrationState::Waiting{ selected }) = state.swarm_migration_state.get_mut(&source) {
                                    *selected = !*selected;
                                }
                            }
                        }
                        _ => {}
                    }
                    Event::Mouse(m) => match m.kind {
                        MouseEventKind::ScrollDown => state.next(),
                        MouseEventKind::ScrollUp => state.previous(),

                        _ => {}
                    }
                    _ => {}
                }
            }
            else => break 'outer BTreeSet::new()
        }

        update_view(&mut terminal, &mut state)?;
    };
    // bye bye
    terminal.clear()?;
    Ok(ret)
}

#[derive(Debug)]
pub enum MigrationState {
    Waiting { selected: bool },
    Found(StreamId),
}
#[derive(Debug)]
struct State {
    table: TableState,
    popup: Option<Popup>,
    swarm_migration_state: BTreeMap<SourceId, MigrationState>,
    known_streams: BTreeSet<StreamId>,
    connected_peers: usize,
}
#[derive(Debug)]
pub enum Popup {
    Help,
    ConfirmSources { sources: BTreeSet<SourceId> },
}

impl State {
    pub fn new(v1_sources: Vec<SourceId>) -> Self {
        let swarm_migration_state: BTreeMap<SourceId, MigrationState> = v1_sources
            .into_iter()
            .map(|s| (s, MigrationState::Waiting { selected: true }))
            .collect();
        let mut state = TableState::default();
        if !swarm_migration_state.is_empty() {
            state.select(Some(0));
        }
        Self {
            table: state,
            popup: None,
            connected_peers: 0,
            known_streams: Default::default(),
            swarm_migration_state,
        }
    }
    pub fn next(&mut self) {
        if self.popup.is_some() {
            return;
        }
        let i = match self.table.selected() {
            Some(i) => {
                if i < self.swarm_migration_state.len() - 1 {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.table.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.popup.is_some() {
            return;
        }
        let i = match self.table.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.table.select(Some(i));
    }
    pub fn first(&mut self) {
        if self.popup.is_some() {
            return;
        }
        if !self.swarm_migration_state.is_empty() {
            self.table.select(Some(0));
        }
    }
    pub fn last(&mut self) {
        if self.popup.is_some() {
            return;
        }
        if !self.swarm_migration_state.is_empty() {
            self.table.select(Some(self.swarm_migration_state.len() - 1));
        }
    }
    pub fn confirm_sources(&mut self) {
        if self.popup.is_some() {
            return;
        }
        let sources: BTreeSet<SourceId> = self
            .swarm_migration_state
            .iter()
            .filter(|(_, state)| matches!(**state, MigrationState::Waiting { selected: true }))
            .map(|(source, _)| *source)
            .collect();
        self.popup = Some(Popup::ConfirmSources { sources })
    }
}
fn update_view(terminal: &mut Terminal<impl Backend>, state: &mut State) -> anyhow::Result<()> {
    tracing::debug!("State {:?}", state);
    let event_emitting_nodes = state
        .known_streams
        .iter()
        .map(|x| x.node_id())
        .collect::<BTreeSet<_>>()
        .len();
    terminal.draw(|f| {
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(20),
                    Constraint::Percentage(75),
                    Constraint::Percentage(5),
                ]
                .as_ref(),
            )
            .margin(5)
            .split(f.size());

        let table = {
            let rows = state.swarm_migration_state.iter().enumerate().map(|(idx, (k, v))| {
                let mut c = vec![idx.to_string(), k.to_string()];
                match v {
                    MigrationState::Waiting { selected } => {
                        c.push("❌ (not migrated)".to_string());
                        if *selected {
                            c.push("✔".into())
                        }
                    }
                    MigrationState::Found(x) => c.push(format!(
                        "✔️ ({}..)",
                        x.node_id().to_string().chars().take(10).collect::<String>()
                    )),
                }

                let cells = c.into_iter().map(Cell::from);
                Row::new(cells)
            });

            let header_cells = ["", "V1 SourceId", "State", "Marked for conversion"]
                .iter()
                .map(|h| Cell::from(*h).style(Style::default().fg(Color::Blue)));
            let table_header = Row::new(header_cells).height(1).bottom_margin(1);
            Table::new(rows)
                .header(table_header)
                .widths(&[
                    Constraint::Percentage(10),
                    Constraint::Percentage(20),
                    Constraint::Percentage(45),
                    Constraint::Percentage(25),
                ])
                .highlight_symbol(">> ")
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .block(Block::default().title("Total v1 sources").borders(Borders::ALL))
        };

        let header = {
            let text = vec![
                Spans::from(vec![format!("Connected v2 peers: {}", state.connected_peers).into()]),
                Spans::from(vec![format!(
                    "Migrated v1 sources: {}/{}",
                    event_emitting_nodes,
                    state.swarm_migration_state.len()
                )
                .into()]),
            ];
            Paragraph::new(text)
                .block(Block::default().title("Migration").borders(Borders::ALL))
                .alignment(Alignment::Left)
        };
        f.render_widget(header, areas[0]);
        f.render_stateful_widget(table, areas[1], &mut state.table);
        f.render_widget(
            Paragraph::new(vec![Spans::from(vec![Span::styled(
                "Actyx v1 migration tool. Press 'h' for help.",
                Style::default().add_modifier(Modifier::BOLD),
            )])]),
            areas[2],
        );

        if let Some(popup) = &state.popup {
            let (header, body) = match popup {
                Popup::Help => (
                    "Help",
                    Text::from(
                        "This view shows the current migration state of all\n\
                        v1 sources within the attached swarm. You can assess the\n\
                        status quo and/or watch nodes migrate live. After you\n\
                        have determined the successful migration of all expected\n\
                        nodes, you can force a local conversion of all\n\
                        \"dead\" nodes. You can select v1 sources for conversion\n\
                        by navigating through the list and marking them with\n\
                        BACKSPACE or SPACE.\n\n\
                        Press 'm' to start the migration.\n\
                        Press ESC to exit this popup.\n\
                        Press 'q' or Ctrl-C to exit the application.",
                    ),
                ),
                Popup::ConfirmSources { sources } => (
                    "Confirm source selection",
                    Text::from(format!(
                        "{} sources have been selected for local conversion.\n\n\
                         Press 'm' to continue. Press ESC to exit.",
                        sources.len(),
                    )),
                ),
            };
            let area = centered_rect(60, 40, f.size());
            let block = Paragraph::new(body)
                .block(Block::default().title(header).borders(Borders::ALL))
                .style(Style::default().fg(Color::White).bg(Color::Black));
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(block, area);
        }
    })?;
    Ok(())
}

/// helper function to create a centered rect using up
/// certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
fn copy_dir_recursive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursive(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
