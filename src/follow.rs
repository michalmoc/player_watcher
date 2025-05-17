use crate::constants::{ACTIVE_PLAYER_PROPERTY, PROPERTIES, PROPERTIES_CHANGED, WELL_KNOWN_PATH};
use dbus::arg::prop_cast;
use dbus::message::MatchRule;
use dbus::nonblock::stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;
use dbus_tokio::connection;
use tokio::signal;

pub async fn follow_changes() -> Result<(), dbus::Error> {
    let (resource, connection) = connection::new_session_sync()?;

    tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    let mr = MatchRule::new_signal(PROPERTIES, PROPERTIES_CHANGED).with_path(WELL_KNOWN_PATH);
    let _m = connection
        .add_match(mr)
        .await?
        .cb(move |_, props: PropertiesPropertiesChanged| {
            if let Some(name) =
                prop_cast::<String>(&props.changed_properties, ACTIVE_PLAYER_PROPERTY)
            {
                println!("{}", name);
            }
            true
        });

    signal::ctrl_c().await.expect("failed to listen for event");

    Ok(())
}
