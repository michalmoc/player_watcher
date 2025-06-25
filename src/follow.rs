use crate::constants::{
    ACTIVE_PLAYER_PROPERTY, MPRIS_PATH, MPRIS_PLAYER_ITF, PROPERTIES, PROPERTIES_CHANGED,
    WELL_KNOWN_PATH,
};
use crate::get::get_active_player_impl;
use dbus::arg;
use dbus::arg::{RefArg, Variant, prop_cast};
use dbus::message::MatchRule;
use dbus::nonblock::stdintf::org_freedesktop_dbus::{Properties, PropertiesPropertiesChanged};
use dbus::nonblock::{MsgMatch, Proxy, SyncConnection};
use dbus_tokio::connection;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::sync::Mutex;

async fn change_metadata(
    changed_properties: arg::PropMap,
    data: Arc<Mutex<Data>>,
) -> Result<(), dbus::Error> {
    let mut data = data.lock().await;

    data.change_metadata(changed_properties);
    println!("{}", data);

    Ok(())
}

// TODO: follow status

async fn listen_for_metadata(
    connection: Arc<SyncConnection>,
    data: Arc<Mutex<Data>>,
) -> Result<(), dbus::Error> {
    let player = data.lock().await.player_name.clone();

    if player != "" {
        let data_clone = data.clone();

        let mr = MatchRule::new_signal(PROPERTIES, PROPERTIES_CHANGED)
            .with_path(MPRIS_PATH)
            .with_sender(player);
        let m = connection
            .add_match(mr)
            .await?
            .cb(move |_, props: PropertiesPropertiesChanged| {
                tokio::spawn(change_metadata(
                    props.changed_properties,
                    data_clone.clone(),
                ));

                true
            });
        let mut data = data.lock().await;
        if let Some(l) = &data.metadata_listen {
            connection.remove_match(l.token()).await?;
        }
        data.metadata_listen = Some(m);
    } else {
        let mut data = data.lock().await;
        if let Some(l) = &data.metadata_listen {
            connection.remove_match(l.token()).await?;
        }
        data.metadata_listen = None;
    }

    Ok(())
}

async fn change_player(
    new_player_name: String,
    connection: Arc<SyncConnection>,
    data: Arc<Mutex<Data>>,
) -> Result<(), dbus::Error> {
    {
        let mut data = data.lock().await;
        data.change_player(new_player_name.clone());

        if new_player_name.is_empty() {
            println!("{}", data);
            return Ok(());
        }
    }

    let proxy = Proxy::new(
        new_player_name,
        MPRIS_PATH,
        Duration::from_secs(5),
        connection.clone(),
    );

    let props = proxy
        .get::<arg::PropMap>(MPRIS_PLAYER_ITF, "Metadata")
        .await?;
    {
        let mut data = data.lock().await;
        data.change_metadata(props);
        println!("{}", data);
    }

    let props = proxy
        .get::<String>(MPRIS_PLAYER_ITF, "PlaybackStatus")
        .await?;
    {
        let mut data = data.lock().await;
        data.change_status(&props);
        println!("{}", data);
    }

    listen_for_metadata(connection.clone(), data.clone()).await?;

    Ok(())
}

async fn listen_for_player_changes(
    connection: Arc<SyncConnection>,
    data: Arc<Mutex<Data>>,
) -> Result<MsgMatch, dbus::Error> {
    let mr = MatchRule::new_signal(PROPERTIES, PROPERTIES_CHANGED).with_path(WELL_KNOWN_PATH);
    let m = connection
        .add_match(mr)
        .await?
        .cb(move |_, props: PropertiesPropertiesChanged| {
            if let Some(name) =
                prop_cast::<String>(&props.changed_properties, ACTIVE_PLAYER_PROPERTY)
            {
                tokio::spawn(change_player(
                    name.clone(),
                    connection.clone(),
                    data.clone(),
                ));
            }
            true
        });

    Ok(m)
}

#[derive(Default)]
struct Data {
    player_name: String,
    metadata_listen: Option<MsgMatch>,

    playing: bool,

    length: Option<i32>,
    album: Option<String>,
    album_artist: Option<String>,
    art_url: Option<String>,
    title: Option<String>,
    track_number: Option<i32>,
    disc_number: Option<i32>,
    url: Option<String>,
    artist: Option<String>,
}

impl Data {
    fn change_player(&mut self, name: String) {
        self.player_name = name;
        self.playing = false;
        self.length = None;
        self.album = None;
        self.album_artist = None;
        self.art_url = None;
        self.title = None;
        self.track_number = None;
        self.disc_number = None;
        self.url = None;
        self.artist = None;
    }

    fn vec_or_str(v: &Variant<Box<dyn RefArg>>) -> String {
        if let Some(v) = v.as_str() {
            v.to_owned()
        } else {
            let v = v
                .as_iter()
                .unwrap()
                .flat_map(|s| s.as_iter().unwrap().map(|s| s.as_str().unwrap().to_owned()))
                .collect::<Vec<_>>();
            v.join(", ")
        }
    }

    fn read_len(v: &Variant<Box<dyn RefArg>>) -> i32 {
        if let Some(v) = v.as_u64() {
            v as i32
        } else if let Some(v) = v.as_i64() {
            v as i32
        } else {
            println!("{:?} = {:?}", v, v.arg_type());
            panic!("unknown type");
        }
    }

    fn escape(s: &str) -> String {
        s.replace("\"", "\\\"")
    }

    fn read_string(v: &Variant<Box<dyn RefArg>>) -> String {
        Self::escape(&Self::vec_or_str(v))
    }

    fn change_metadata(&mut self, props: arg::PropMap) {
        if let Some(playback) = arg::prop_cast::<String>(&props, "PlaybackStatus") {
            self.change_status(playback);
            return;
        }

        let props = if let Some(metadata) = arg::prop_cast::<arg::PropMap>(&props, "Metadata") {
            metadata
        } else {
            &props
        };

        for (prop, value) in props {
            match prop.as_str() {
                "mpris:length" => self.length = Some(Self::read_len(value)),
                "xesam:album" => self.album = Some(Self::read_string(value)),
                "xesam:albumArtist" => self.album_artist = Some(Self::read_string(value)),
                "mpris:artUrl" => self.art_url = Some(Self::read_string(value)),
                "xesam:title" => self.title = Some(Self::read_string(value)),
                "xesam:trackNumber" => self.track_number = Some(value.as_i64().unwrap() as i32),
                "xesam:discNumber" => self.disc_number = Some(value.as_i64().unwrap() as i32),
                "xesam:url" => self.url = Some(Self::read_string(value)),
                "xesam:artist" => self.artist = Some(Self::read_string(value)),
                _ => (),
            }
        }
    }

    fn change_status(&mut self, status: &str) {
        self.playing = status == "Playing";
    }
}

impl Display for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let length = (&self.length).map(|s| s.to_string()).unwrap_or_default();
        let album = (&self.album).as_ref().map(|s| s.as_str()).unwrap_or("");
        let album_artist = (&self.album_artist)
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");
        let art_url = (&self.art_url).as_ref().map(|s| s.as_str()).unwrap_or("");
        let title = (&self.title).as_ref().map(|s| s.as_str()).unwrap_or("");
        let track_number = (&self.track_number)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let disc_number = (&self.disc_number)
            .map(|s| s.to_string())
            .unwrap_or_default();
        let url = (&self.url).as_ref().map(|s| s.as_str()).unwrap_or("");
        let artist = (&self.artist).as_ref().map(|s| s.as_str()).unwrap_or("");

        write!(
            f,
            "\
        {{ \
        \"player\": \"{}\", \
        \"playing\": \"{}\", \
        \"length\": \"{}\", \
        \"album\": \"{}\", \
        \"album_artist\": \"{}\", \
        \"art_url\": \"{}\", \
        \"title\": \"{}\", \
        \"track_number\": \"{}\", \
        \"disc_number\": \"{}\", \
        \"url\": \"{}\", \
        \"artist\": \"{}\" \
         }}\
        ",
            self.player_name,
            self.playing,
            length,
            album,
            album_artist,
            art_url,
            title,
            track_number,
            disc_number,
            url,
            artist,
        )
    }
}

pub async fn follow_changes() -> Result<(), dbus::Error> {
    let (resource, connection) = connection::new_session_sync()?;

    tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    let data = Arc::new(Mutex::new(Data::default()));
    let (player, _) = get_active_player_impl(connection.clone()).await?;
    change_player(player, connection.clone(), data.clone()).await?;

    let player_listen = listen_for_player_changes(connection.clone(), data.clone()).await?;

    signal::ctrl_c().await.expect("failed to listen for event");
    println!("ctrlc");

    connection.remove_match(player_listen.token()).await?;

    if let Some(l) = &data.lock().await.metadata_listen {
        connection.remove_match(l.token()).await?;
    }

    Ok(())
}
