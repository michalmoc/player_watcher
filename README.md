# About

Daemon tracking current media player via MPRIS. The current player
is defined as the player which started playing the last.

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

or follow it (whenever active player changes, a new line with its name
will be printed):

```shell
$ player_watcher follow
org.mpris.MediaPlayer2.spotify
org.mpris.MediaPlayer2.firefox.instance_1_79
...
```

You can also switch current player:

```shell
$ player_watcher shift
$ player_watcher unshift
```

There is also a simple wrapper for playerctl:

```shell
$ playerctl_auto play-pause
```

# Why

So I can use `playerctl` or similar tools on the exact player I'm using now,
not the one which just happened to be opened later or earlier.

Project inspired by `playerctld` which uses inconvenient definition of
current player and is written in pure C and gtk.