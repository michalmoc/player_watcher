# About

Daemon tracking current media player via MPRIS, and a client to control the player.

Features smart tracking of the current player, controlled by advanced AI model (i.e. a few boolean expressions).

# Usage

Start daemon via

```sh
$ player_watcher daemon
```

then you can check the current player:

```shell
$ player_watcher get
org.mpris.MediaPlayer2.spotify
```

or follow it (whenever active player or metadata changes, a new line will be printed):

```shell
$ player_watcher follow
{ "player": "org.mpris.MediaPlayer2.spotify", "playing": "true", "length": "328160000", "album": "Korngold: Die tote Stadt", "album_artist": "Erich Wolfgang Korngold", "art_url": "https://i.scdn.co/image/ab67616d0000b273e39c46a035ab6f7346a7e3e5", "title": "Die tote Stadt (The Dead City), Op. 12: Act I Scene 5: Gluck, das mir verblieb (Marietta, Paul)", "track_number": "6", "disc_number": "1", "url": "https://open.spotify.com/track/47xZ59XjNaGgnmWy2X1WUL", "artist": "Erich Wolfgang Korngold" }
...
```

You can also switch or controll current player:

```shell
$ player_watcher shift
$ player_watcher unshift
$ player_watcher play-pause
```

# Format of following

`player_watcher follow` will, on each change, print one line of JSON. This is a simple object with following fields:

* `"player"`: dbus address of current player. You can extract name from it or use to send custom dbus commands.
* `"playing"`: true or false, whether the player is currently playing
* `"length"`: track length as reported by player
* `"album"`: album name
* `"album_artist"`: album artists
* `"art_url"`: url f cover art, which should be a downloadable image
* `"title"`: track title
* `"track_number"`: track number
* `"disc_number"`: disc number
* `"url"`: url reported by player, which may be used to open it
* `"artist"`: track artists

# Eww example

Fragment of my eww config:

```lisp
(deflisten music_meta :initial "{}"
  "scripts/music")
(defwidget music []
  (revealer
      :reveal {music_meta != ""}
    (eventbox
        :onclick "player_watcher play-pause"
      (tooltip
        (music_desc)
        (box :orientation "v" :space-evenly "false"
          (label
            :gravity "east"
            :angle 270
            :class "music-icon"
            :text "${music_meta['playing'] ? "" : ""}")
          (label
            :gravity "east"
            :angle 270
            :class "music-text"
            :limit-width "40"
            :show-truncated "true"
            :markup "${music_meta['artist']} - ${music_meta['title']}"))))))
(defwidget music_desc []
  (box :orientation "v" :space-evenly false :class "music-desc"
    (image :class "music-desc-cover" :image-width 264 :image-height 264 :path "${music_meta['art_url']}")
    (label :halign "start" :limit-width "40" :class "music-desc-title" :markup "${music_meta['title']}")
    (label :halign "start" :limit-width "40" :markup "${music_meta['album']}")
    (label :halign "start" :limit-width "40" :markup "${music_meta['artist']}")))
```

and the script `music`, which serves only to download the cover image:

```bash
#!/bin/bash

# saves covers here
Cover=/tmp/cover.png
# if cover not found in metadata use this instead
bkpCover=~/.config/eww/assets/fallback.png

fetch_cover() {
    while read -r line; do
        new_cover=$(echo "$line" | jq -r -c '.["art_url"]')
        if [ -z "$new_cover" ]; then
            cp "$bkpCover" "$Cover"
        else
            curl --output $Cover $new_cover || cp "$bkpCover" "$Cover"
        fi
        echo $line | jq ".[\"art_url\"] = \"$Cover\"" -c
    done
}

player_watcher follow | fetch_cover
```

# Why

So I can check and control the exact player I'm using now,
not the one which just happened to be opened later or earlier.
Project is very similar to playerctl, but with strongly integrated concept of
"current player".