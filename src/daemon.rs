use crate::constants::{
    ACTIVE_PLAYER_PROPERTY, DBUS, MPRIS_PATH, PROPERTIES, PROPERTIES_CHANGED, SHIFT_METHOD,
    UNSHIFT_METHOD, WELL_KNOWN_NAME, WELL_KNOWN_PATH,
};
use crate::players::{Players, is_player};
use dbus::Message;
use dbus::arg::{RefArg, Variant, prop_cast};
use dbus::channel::Sender;
use dbus::message::MatchRule;
use dbus::nonblock::stdintf::org_freedesktop_dbus::{
    PropertiesPropertiesChanged, RequestNameReply,
};
use dbus::nonblock::{MsgMatch, SyncConnection};
use dbus_tokio::connection;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use tokio::sync::Notify;

#[derive(Default)]
struct PlayersQueue {
    players: Players,
    queue: Vec<Arc<str>>,
}

impl PlayersQueue {
    pub async fn find_existing(
        &mut self,
        connection: Arc<SyncConnection>,
    ) -> Result<(), dbus::Error> {
        self.queue = self.players.find_existing(connection).await?;

        Ok(())
    }

    fn get_active(&self) -> Option<Arc<str>> {
        self.queue.first().cloned()
    }

    fn add_player(&mut self, name: Arc<str>, channels: Vec<Arc<str>>) {
        if self.players.add(name.clone(), channels) {
            if self.queue.is_empty() {
                self.queue.push(name.clone());
            } else {
                self.queue.insert(0, name);
            }
        }
    }

    fn remove_player(&mut self, name: Arc<str>) {
        self.players.remove(name.clone());
        self.queue.retain(|e| e != &name);
    }

    fn find_by_channel(&self, channel: &str) -> Option<Arc<str>> {
        self.players.find_by_channel(channel)
    }

    fn promote(&mut self, name: Arc<str>) {
        self.queue.retain(|e| e != &name);
        self.queue.insert(0, name);
    }

    fn shift(&mut self) {
        if !self.queue.is_empty() {
            self.queue.rotate_right(1);
        }
    }

    fn unshift(&mut self) {
        if !self.queue.is_empty() {
            self.queue.rotate_left(1);
        }
    }

    pub fn get_channels(&self, name: Arc<str>) -> Option<HashSet<Arc<str>>> {
        self.players.get_channels(name).cloned()
    }
}

#[derive(Clone)]
pub struct Daemon {
    connection: Arc<SyncConnection>,
    players: Arc<RwLock<PlayersQueue>>,
    players_changed: Arc<Notify>,
}

impl Daemon {
    pub async fn new() -> Result<Self, dbus::Error> {
        let players = Arc::new(RwLock::new(PlayersQueue::default()));
        let players_changed = Arc::new(Notify::new());

        let (resource, connection) = connection::new_session_sync()?;

        tokio::spawn(async {
            let err = resource.await;
            panic!("Lost connection to D-Bus: {}", err);
        });

        let reply = connection
            .request_name(WELL_KNOWN_NAME, false, false, true)
            .await?;

        if reply != RequestNameReply::PrimaryOwner {
            panic!("Already running");
        }

        Ok(Self {
            connection,
            players,
            players_changed,
        })
    }

    pub async fn run(&mut self) -> Result<(), dbus::Error> {
        let m0 = self
            .listen_for_player_changes(self.connection.clone())
            .await?;
        self.players
            .write()
            .unwrap()
            .find_existing(self.connection.clone())
            .await?;
        self.players_changed.notify_one();
        let m1 = self.listen_for_property_gets().await?;
        let m2 = self.listen_for_methods().await?;
        let m3 = self.listen_for_status_changes().await?;
        self.await_player_queue_changes().await;

        // unreachable, await_player_queue_changes() is endless loop

        self.connection.remove_match(m3.token()).await?;
        self.connection.remove_match(m2.token()).await?;
        self.connection.remove_match(m1.token()).await?;
        self.connection.remove_match(m0.token()).await?;

        unreachable!()
    }

    pub async fn listen_for_player_changes(
        &self,
        connection: Arc<SyncConnection>,
    ) -> Result<MsgMatch, dbus::Error> {
        let cloned = self.clone();
        let mr = MatchRule::new_signal(DBUS, "NameOwnerChanged");
        let m = connection.add_match(mr).await?.cb(
            move |_, (name, old_owner, new_owner): (String, String, String)| {
                if is_player(&name) {
                    assert_ne!(old_owner.is_empty(), new_owner.is_empty());
                    println!("new player {:?}", new_owner);

                    let name: Arc<str> = name.into();
                    if old_owner.is_empty() {
                        cloned
                            .players
                            .write()
                            .unwrap()
                            .add_player(name.clone(), vec![new_owner.into()]);
                        // TODO: check playback status
                    } else {
                        cloned.players.write().unwrap().remove_player(name.clone());
                    }
                    cloned.players_changed.notify_one();
                }
                true
            },
        );

        Ok(m)
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
                .unwrap_or_default();

            if active != last_sent {
                last_sent = active.clone();

                let active: Box<(dyn RefArg + 'static)> = Box::new(active.to_string());
                let props = PropertiesPropertiesChanged {
                    interface_name: WELL_KNOWN_NAME.to_string(),
                    changed_properties: HashMap::from([(
                        ACTIVE_PLAYER_PROPERTY.to_string(),
                        Variant(active),
                    )]),
                    invalidated_properties: vec![],
                };
                let mut msg =
                    Message::new_signal(WELL_KNOWN_PATH, PROPERTIES, PROPERTIES_CHANGED).unwrap();
                msg.append_all(props);

                self.connection.send(msg).unwrap();
            }
        }
    }

    async fn listen_for_status_changes(&self) -> Result<MsgMatch, dbus::Error> {
        let daemon = self.clone();
        let mr = MatchRule::new_signal(PROPERTIES, PROPERTIES_CHANGED).with_path(MPRIS_PATH);
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

    async fn listen_for_property_gets(&self) -> Result<MsgMatch, dbus::Error> {
        let daemon = self.clone();
        let mr = MatchRule::new_method_call()
            .with_path(WELL_KNOWN_PATH)
            .with_interface(PROPERTIES)
            .with_member("Get");

        let m = self
            .connection
            .add_match(mr)
            .await?
            .cb(move |req, (_, name): (String, String)| {
                if name == ACTIVE_PLAYER_PROPERTY {
                    let resp = daemon
                        .players
                        .read()
                        .unwrap()
                        .get_active()
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    let channels = daemon
                        .players
                        .read()
                        .unwrap()
                        .get_channels(resp.clone().into());
                    let msg = if let Some(channels) = channels {
                        Message::new_method_return(&req).unwrap().append1(Variant((
                            resp,
                            Vec::from_iter(channels.iter().map(|s| s.to_string())),
                        )))
                    } else {
                        Message::new_method_return(&req)
                            .unwrap()
                            .append1(Variant((resp, Vec::<String>::new())))
                    };
                    daemon.connection.send(msg).unwrap();
                }

                true
            });

        Ok(m)
    }

    async fn listen_for_methods(&self) -> Result<MsgMatch, dbus::Error> {
        let daemon = self.clone();
        let mr = MatchRule::new_method_call()
            .with_path(WELL_KNOWN_PATH)
            .with_interface(WELL_KNOWN_NAME);

        let m = self.connection.add_match(mr).await?.cb(move |req, (): ()| {
            if let Some(command) = req.member() {
                if command.deref() == SHIFT_METHOD {
                    daemon.players.write().unwrap().shift();
                    daemon.players_changed.notify_one();
                } else if command.deref() == UNSHIFT_METHOD {
                    daemon.players.write().unwrap().unshift();
                    daemon.players_changed.notify_one();
                }
            }

            true
        });

        Ok(m)
    }
}

// TODO: new channels may be opened, would be nice to listen for it
