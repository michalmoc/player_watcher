use crate::constants::{MPRIS_PATH, MPRIS_PLAYER_ITF};
use crate::get::get_active_player_impl;
use dbus::nonblock::Proxy;
use dbus_tokio::connection;
use std::time::Duration;

pub async fn control(command: &str) -> Result<(), Box<dbus::Error>> {
    let (resource, connection) = connection::new_session_sync()?;
    tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    let (player, _) = get_active_player_impl(connection.clone()).await?;

    let proxy = Proxy::new(&player, MPRIS_PATH, Duration::from_secs(5), connection);

    let _prop: () = proxy.method_call(MPRIS_PLAYER_ITF, command, ()).await?;

    Ok(())
}
