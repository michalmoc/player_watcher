use crate::constants::{
    ACTIVE_PLAYER_PROPERTY, DBUS, MPRIS_PATH, MPRIS_PLAYER_ITF, MPRIS_PREFIX, PROPERTIES,
    PROPERTIES_CHANGED, SHIFT_METHOD, UNSHIFT_METHOD, WELL_KNOWN_NAME, WELL_KNOWN_PATH,
};
use crate::players::Players;
use crate::players_queue::PlayersQueue;
use dbus::Message;
use dbus::arg::{RefArg, Variant, prop_cast};
use dbus::channel::Sender;
use dbus::message::MatchRule;
use dbus::nonblock::stdintf::org_freedesktop_dbus::{
    Properties, PropertiesPropertiesChanged, RequestNameReply,
};
use dbus::nonblock::{MsgMatch, Proxy, SyncConnection};
use dbus_tokio::connection;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::signal;

fn is_player(name: &str) -> bool {
    name.starts_with(MPRIS_PREFIX)
}

#[derive(Clone)]
pub struct Daemon {
    connection: Arc<SyncConnection>,
    players: Arc<RwLock<Players>>,
    queue: Arc<RwLock<PlayersQueue>>,
}

impl Daemon {
    pub async fn new() -> Result<Self, dbus::Error> {
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

        let players = Arc::new(RwLock::new(Players::default()));
        let queue = Arc::new(RwLock::new(PlayersQueue::default()));

        Ok(Self {
            connection,
            players,
            queue,
        })
    }

    pub async fn run(&mut self) -> Result<(), dbus::Error> {
        let m0 = self.listen_for_player_changes().await?;
        self.find_existing(self.connection.clone()).await?;

        let m1 = self.listen_for_property_gets().await?;
        let m2 = self.listen_for_methods().await?;
        let m3 = self.listen_for_status_changes().await?;

        signal::ctrl_c().await.expect("failed to listen for event");

        self.connection.remove_match(m3.token()).await?;
        self.connection.remove_match(m2.token()).await?;
        self.connection.remove_match(m1.token()).await?;
        self.connection.remove_match(m0.token()).await?;

        Ok(())
    }

    async fn find_existing(&mut self, connection: Arc<SyncConnection>) -> Result<(), dbus::Error> {
        let proxy = Proxy::new(DBUS, "/", Duration::from_secs(5), connection);

        let (names,): (Vec<String>,) = proxy.method_call(DBUS, "ListNames", ()).await?;

        for name in names {
            if is_player(&name) {
                let (owners,): (Vec<String>,) = proxy
                    .method_call(DBUS, "ListQueuedOwners", (&name,))
                    .await?;
                println!("old player {:?}", name);
                self.add(name, owners.into_iter().map(Into::into).collect())
                    .await?;
            }
        }

        Ok(())
    }

    async fn add(&mut self, name: String, channels: Vec<Arc<str>>) -> Result<(), dbus::Error> {
        let name: Arc<str> = name.into();
        let playing = self.check_if_playing(&name).await?;

        self.players.write().unwrap().add(name.clone(), channels);
        self.queue.write().unwrap().add_player(name, playing);
        self.notify_of_new_active();

        Ok(())
    }

    async fn remove(&mut self, name: &str) -> Result<(), dbus::Error> {
        self.players.write().unwrap().remove(name);
        self.queue.write().unwrap().remove_player(name);
        self.notify_of_new_active();

        Ok(())
    }

    async fn check_if_playing(&self, player: &str) -> Result<bool, dbus::Error> {
        let proxy = Proxy::new(
            player,
            MPRIS_PATH,
            Duration::from_secs(5),
            self.connection.clone(),
        );

        let props = proxy
            .get::<String>(MPRIS_PLAYER_ITF, "PlaybackStatus")
            .await?;

        match props.as_str() {
            "Playing" => Ok(true),
            _ => Ok(false),
        }
    }

    pub async fn listen_for_player_changes(&self) -> Result<MsgMatch, dbus::Error> {
        let daemon = self.clone();

        let mr = MatchRule::new_signal(DBUS, "NameOwnerChanged");
        let m = self.connection.add_match(mr).await?.cb(
            move |_, (name, old_owner, new_owner): (String, String, String)| {
                if is_player(&name) {
                    assert_ne!(old_owner.is_empty(), new_owner.is_empty());
                    println!("new player {:?}", new_owner);

                    let mut clone = daemon.clone();
                    tokio::spawn(async move {
                        if old_owner.is_empty() {
                            clone.add(name, vec![new_owner.into()]).await.unwrap();
                        } else {
                            clone.remove(&name).await.unwrap();
                        }
                    });
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
                    let players = daemon.players.read().unwrap();
                    let queue = daemon.queue.read().unwrap();

                    let resp = queue
                        .get_active()
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    let channels = players.get_channels(resp.clone().into());

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
                    daemon.queue.write().unwrap().shift();
                } else if command.deref() == UNSHIFT_METHOD {
                    daemon.queue.write().unwrap().unshift();
                }
                daemon.notify_of_new_active();
            }

            true
        });

        Ok(m)
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
                    if daemon.queue.write().unwrap().promote(player) {
                        daemon.notify_of_new_active();
                    }
                } else if daemon.queue.write().unwrap().demote(player) {
                    daemon.notify_of_new_active();
                }

                true
            },
        );

        Ok(m)
    }

    fn notify_of_new_active(&self) {
        let active = self.queue.read().unwrap().get_active().unwrap_or_default();

        let active: Box<(dyn RefArg + 'static)> = Box::new(active.to_string());
        let props = PropertiesPropertiesChanged {
            interface_name: WELL_KNOWN_NAME.to_string(),
            changed_properties: HashMap::from([(
                ACTIVE_PLAYER_PROPERTY.to_string(),
                Variant(active),
            )]),
            invalidated_properties: vec![],
        };
        let mut msg = Message::new_signal(WELL_KNOWN_PATH, PROPERTIES, PROPERTIES_CHANGED).unwrap();
        msg.append_all(props);

        self.connection.send(msg).unwrap();
    }
}

// TODO: new channels may be opened, would be nice to listen for it
