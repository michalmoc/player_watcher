# About

Daemon tracking current media player via MPRIS, and a client to control the player.

The current player is defined as the player which started playing the last.

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
{ "player": "org.mpris.MediaPlayer2.spotify", "length": "117333000", "album": "Mozart: Così fan tutte", "album_artist": "Wolfgang Amadeus Mozart", "art_url": "https://i.scdn.co/image/ab67616d0000b273599d2e3b7701daab4df10656", "title": "Così fan tutte, K.588 / Act 1: \"Al fato dan legge\" - \"La commedia è graziosa\"", "track_number": "8", "disc_number": "1", "url": "https://open.spotify.com/track/3cdUev0jFsyOIQFmLLroVw", "artist": "Wolfgang Amadeus Mozart" }
...
```

You can also switch or controll current player:

```shell
$ player_watcher shift
$ player_watcher unshift
$ player_watcher play-pause
```

# Why

So I can check and control the exact player I'm using now,
not the one which just happened to be opened later or earlier.
Project is very similar to playerctl, but with strongly integrated concept of
"current player".