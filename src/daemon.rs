use crate::constants::{
    ACTIVE_CHANGED_SIGNAL, DBUS, MPRIS_PATH, MPRIS_PREFIX, PROPERTIES, WELL_KNOWN_NAME,
};
use dbus::Message;
use dbus::arg::prop_cast;
use dbus::channel::Sender;
use dbus::message::MatchRule;
use dbus::nonblock::stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;
use dbus::nonblock::{MsgMatch, Proxy, SyncConnection};
use dbus_tokio::connection;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Notify;

fn is_player(name: &str) -> bool {
    name.starts_with(MPRIS_PREFIX)
}

#[derive(Default)]
struct Players {
    players: HashMap<Arc<str>, HashSet<Arc<str>>>,
    rev_players: HashMap<Arc<str>, Arc<str>>,
    queue: Vec<Arc<str>>,
}

impl Players {
    fn get_active(&self) -> Option<&Arc<str>> {
        self.queue.first()
    }

    fn add_player(&mut self, name: Arc<str>, channels: Vec<Arc<str>>) {
        for channel in &channels {
            self.rev_players.insert(channel.clone(), name.clone());
        }
        if self
            .players
            .insert(name.clone(), HashSet::from_iter(channels))
            .is_none()
        {
            if self.queue.is_empty() {
                self.queue.push(name.clone());
            } else {
                self.queue.insert(1, name);
            }
        }
    }

    fn remove_player(&mut self, name: Arc<str>) {
        if let Some(channels) = self.players.remove(&name) {
            for channel in channels {
                self.rev_players.remove(&channel);
            }

            self.queue.retain(|e| e != &name);
        }
    }

    fn find_by_channel(&self, channel: &str) -> Option<&Arc<str>> {
        self.rev_players.get(channel)
    }

    fn promote(&mut self, name: Arc<str>) {
        self.queue.retain(|e| e != &name);
        self.queue.insert(0, name);
    }
}

#[derive(Clone)]
pub struct Daemon {
    connection: Arc<SyncConnection>,
    players: Arc<RwLock<Players>>,
    players_changed: Arc<Notify>,
}

impl Daemon {
    pub async fn new() -> Result<Self, dbus::Error> {
        let players = Arc::new(RwLock::new(Players::default()));
        let players_changed = Arc::new(Notify::new());

        let (resource, connection) = connection::new_session_sync()?;

        tokio::spawn(async {
            let err = resource.await;
            panic!("Lost connection to D-Bus: {}", err);
        });

        connection
            .request_name(WELL_KNOWN_NAME, false, false, false)
            .await?;

        Ok(Self {
            connection,
            players,
            players_changed,
        })
    }

    pub async fn run(&self) -> Result<(), dbus::Error> {
        self.listen_for_player_changes().await?;
        self.find_existing_players().await?;
        let _m1 = self.await_player_changes().await?;
        let _m2 = self.await_player_queue_changes().await;
        unreachable!()
    }

    async fn find_existing_players(&self) -> Result<(), dbus::Error> {
        let proxy = Proxy::new(DBUS, "/", Duration::from_secs(5), self.connection.clone());

        let (names,): (Vec<String>,) = proxy.method_call(DBUS, "ListNames", ()).await?;

        for name in names {
            if is_player(&name) {
                let (owners,): (Vec<String>,) = proxy
                    .method_call(DBUS, "ListQueuedOwners", (&name,))
                    .await?;
                self.players
                    .write()
                    .unwrap()
                    .add_player(name.into(), owners.into_iter().map(Into::into).collect());
                self.players_changed.notify_one();
            }
        }

        Ok(())
    }

    async fn await_player_queue_changes(&self) {
        let mut last_sent = Arc::<str>::from("");

        loop {
            self.players_changed.notified().await;

            let active = self
                .players
                .read()
                .unwrap()
                .get_active()
                .cloned()
                .unwrap_or_default();

            if active != last_sent {
                last_sent = active.clone();
                let msg = Message::new_signal("/", WELL_KNOWN_NAME, ACTIVE_CHANGED_SIGNAL)
                    .unwrap()
                    .append1(active.to_string());

                self.connection.send(msg).unwrap();
            }
        }
    }

    async fn listen_for_player_changes(&self) -> Result<MsgMatch, dbus::Error> {
        let daemon = self.clone();
        let mr = MatchRule::new_signal(DBUS, "NameOwnerChanged");
        let m = self.connection.add_match(mr).await?.cb(
            move |_, (name, old_owner, new_owner): (String, String, String)| {
                if is_player(&name) {
                    assert_ne!(old_owner.is_empty(), new_owner.is_empty());
                    if old_owner.is_empty() {
                        daemon
                            .players
                            .write()
                            .unwrap()
                            .add_player(name.into(), vec![new_owner.into()]);
                        daemon.players_changed.notify_one();
                    } else {
                        daemon.players.write().unwrap().remove_player(name.into());
                        daemon.players_changed.notify_one();
                    }
                }
                true
            },
        );

        Ok(m)
    }

    async fn await_player_changes(&self) -> Result<MsgMatch, dbus::Error> {
        let daemon = self.clone();
        let mr = MatchRule::new_signal(PROPERTIES, "PropertiesChanged").with_path(MPRIS_PATH);
        let m = self.connection.add_match(mr).await?.cb(
            move |msg, props: PropertiesPropertiesChanged| {
                let mut player = Arc::<str>::from("");
                let mut status = String::new();

                if let Some(sender) = msg.sender() {
                    if let Some(player_) = daemon.players.read().unwrap().find_by_channel(&sender) {
                        if let Some(status_) =
                            prop_cast::<String>(&props.changed_properties, "PlaybackStatus")
                        {
                            player = player_.clone();
                            status = status_.clone();
                        }
                    }
                }

                // must be outside `if let` to prevent deadlock of daemon.players
                if status == "Playing" {
                    daemon.players.write().unwrap().promote(player);
                    daemon.players_changed.notify_one();
                }

                true
            },
        );

        Ok(m)
    }
}

// TODO: new channels may be opened, would be nice to listen for it
