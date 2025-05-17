use crate::constants::{ACTIVE_PLAYER_PROPERTY, WELL_KNOWN_NAME, WELL_KNOWN_PATH};
use dbus::nonblock::Proxy;
use dbus::nonblock::stdintf::org_freedesktop_dbus::Properties;
use dbus_tokio::connection;
use std::time::Duration;

pub async fn get_active_player() -> Result<(), dbus::Error> {
    let (resource, connection) = connection::new_session_sync()?;

    tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    let proxy = Proxy::new(
        WELL_KNOWN_NAME,
        WELL_KNOWN_PATH,
        Duration::from_secs(5),
        connection,
    );

    let prop = proxy
        .get::<String>(WELL_KNOWN_NAME, ACTIVE_PLAYER_PROPERTY)
        .await?;

    println!("{}", prop);

    Ok(())
}
