use crate::constants::{ACTIVE_CHANGED_SIGNAL, WELL_KNOWN_NAME};
use dbus::message::MatchRule;
use dbus_tokio::connection;
use tokio::signal;

pub async fn follow_changes() -> Result<(), dbus::Error> {
    let (resource, connection) = connection::new_session_sync()?;

    tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    let mr = MatchRule::new_signal(WELL_KNOWN_NAME, ACTIVE_CHANGED_SIGNAL);
    let _m = connection
        .add_match(mr)
        .await?
        .cb(move |_, (name,): (String,)| {
            println!("{}", name);
            true
        });

    signal::ctrl_c().await.expect("failed to listen for event");

    Ok(())
}
