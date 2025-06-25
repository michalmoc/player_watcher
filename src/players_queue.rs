use std::collections::HashSet;
use std::sync::Arc;

#[derive(Default)]
pub struct PlayersQueue {
    playing: HashSet<Arc<str>>,
    queue: Vec<Arc<str>>,
}

impl PlayersQueue {
    pub fn get_active(&self) -> Option<Arc<str>> {
        self.queue.first().cloned()
    }

    pub fn set_playing(&mut self, player: Arc<str>, playing: bool) {
        if playing {
            self.playing.insert(player);
        } else {
            self.playing.remove(&player);
        }
    }

    pub fn add_player(&mut self, name: Arc<str>, playing: bool) {
        if self.queue.is_empty() || playing || !self.playing.contains(&self.queue[0]) {
            self.queue.insert(0, name.clone());
        } else {
            self.queue.insert(1, name.clone());
        }

        self.set_playing(name, playing);
    }

    pub fn remove_player(&mut self, name: &str) {
        self.queue.retain(|e| e.as_ref() != name);
        self.playing.remove(name);
    }

    #[must_use]
    pub fn promote(&mut self, name: Arc<str>) -> bool {
        if let Some(idx) = self.queue.iter().rposition(|n| *n == name) {
            self.set_playing(name, true);
            if idx > 0 {
                self.queue[0..=idx].rotate_right(1);
                true
            } else {
                false
            }
        } else {
            eprintln!("player for promotion not found");
            false
        }
    }

    #[must_use]
    pub fn demote(&mut self, name: Arc<str>) -> bool {
        self.set_playing(name, false);
        if let Some(idx) = self.queue.iter().position(|n| self.playing.contains(n)) {
            self.queue[0..=idx].rotate_right(1);
            true
        } else {
            false
        }
    }

    pub fn shift(&mut self) {
        if !self.queue.is_empty() {
            self.queue.rotate_left(1);
        }
    }

    pub fn unshift(&mut self) {
        if !self.queue.is_empty() {
            self.queue.rotate_right(1);
        }
    }

    // async fn check_all_if_playing(
    //     &mut self,
    //     connection: Arc<SyncConnection>,
    // ) -> Result<(), dbus::Error> {
    //     for player in self.players.iter().cloned().collect::<Vec<_>>() {
    //         self.check_if_playing(player.clone(), connection.clone())
    //             .await?;
    //     }
    //     Ok(())
    // }
    //
}
