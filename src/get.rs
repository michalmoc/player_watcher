use crate::constants::{ACTIVE_PLAYER_PROPERTY, WELL_KNOWN_NAME, WELL_KNOWN_PATH};
use dbus::nonblock::Proxy;
use dbus::nonblock::SyncConnection;
use dbus::nonblock::stdintf::org_freedesktop_dbus::Properties;
use dbus_tokio::connection;
use std::sync::Arc;
use std::time::Duration;

pub async fn get_active_player_impl(
    connection: Arc<SyncConnection>,
) -> Result<(String, Vec<String>), dbus::Error> {
    let proxy = Proxy::new(
        WELL_KNOWN_NAME,
        WELL_KNOWN_PATH,
        Duration::from_secs(5),
        connection,
    );

    let (name, channels) = proxy
        .get::<(String, Vec<String>)>(WELL_KNOWN_NAME, ACTIVE_PLAYER_PROPERTY)
        .await?;

    Ok((name, channels))
}

pub async fn get_active_player() -> Result<(), dbus::Error> {
    let (resource, connection) = connection::new_session_sync()?;
    tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    let (player, _) = get_active_player_impl(connection).await?;

    println!("{}", player);
    Ok(())
}
