use crate::constants::{DBUS, MPRIS_PREFIX};
use dbus::nonblock::Proxy;
use dbus::nonblock::SyncConnection;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

pub fn is_player(name: &str) -> bool {
    name.starts_with(MPRIS_PREFIX)
}

#[derive(Default)]
pub struct Players {
    players: HashMap<Arc<str>, HashSet<Arc<str>>>,
    rev_players: HashMap<Arc<str>, Arc<str>>,
}

impl Players {
    /// return whether this is a new name
    pub fn add(&mut self, name: Arc<str>, channels: Vec<Arc<str>>) -> bool {
        for channel in &channels {
            self.rev_players.insert(channel.clone(), name.clone());
        }

        self.players
            .insert(name.clone(), HashSet::from_iter(channels))
            .is_none()
    }

    pub fn remove(&mut self, name: Arc<str>) {
        if let Some(channels) = self.players.remove(&name) {
            for channel in channels {
                self.rev_players.remove(&channel);
            }
        }
    }

    pub fn find_by_channel(&self, channel: &str) -> Option<Arc<str>> {
        self.rev_players.get(channel).cloned()
    }

    pub async fn find_existing(
        &mut self,
        connection: Arc<SyncConnection>,
    ) -> Result<Vec<Arc<str>>, dbus::Error> {
        let proxy = Proxy::new(DBUS, "/", Duration::from_secs(5), connection);

        let (names,): (Vec<String>,) = proxy.method_call(DBUS, "ListNames", ()).await?;

        for name in names {
            if is_player(&name) {
                let (owners,): (Vec<String>,) = proxy
                    .method_call(DBUS, "ListQueuedOwners", (&name,))
                    .await?;
                println!("old player {:?}", name);
                self.add(name.into(), owners.into_iter().map(Into::into).collect());
            }
        }

        Ok(self.players.keys().cloned().collect())
    }

    pub fn get_channels(&self, name: Arc<str>) -> Option<&HashSet<Arc<str>>> {
        self.players.get(&name)
    }
}
