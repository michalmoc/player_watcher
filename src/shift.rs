use crate::constants::{SHIFT_METHOD, UNSHIFT_METHOD, WELL_KNOWN_NAME, WELL_KNOWN_PATH};
use dbus::nonblock::{MethodReply, Proxy};
use dbus_tokio::connection;
use std::time::Duration;

pub async fn next_player() -> Result<(), dbus::Error> {
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

    let _: MethodReply<()> = proxy.method_call(WELL_KNOWN_NAME, SHIFT_METHOD, ());

    Ok(())
}

pub async fn previous_player() -> Result<(), dbus::Error> {
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

    let _: MethodReply<()> = proxy.method_call(WELL_KNOWN_NAME, UNSHIFT_METHOD, ());

    Ok(())
}
