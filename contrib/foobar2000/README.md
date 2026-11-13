# foobar2000 (Wine) in KDE media controls

Use this to control foobar2000 running under Wine from KDE's media player widget and your keyboard's media keys. KDE also shows the current track and whether it's playing. Both parts start automatically when you log in.

How it works:

```
KDE media widget / media keys
  → MPRIS → media-control-bridge mpris-client   (player named "foobar2000")
  → 127.0.0.1:17777 → media-control-bridge serve
      buttons: wine foobar2000.exe /playpause, /next, ...   (inside foobar's Wine prefix)
      status:  foo_beefweb HTTP API (title, artist, album, play state)
```

## 1. Requirements

- `curl` and `jq`, which the track info needs.
- The **foo_beefweb** component installed in foobar2000. Download it from https://github.com/hyperblast/beefweb/releases, then in foobar2000 use *File → Preferences → Components → Install…* and restart foobar2000. It serves on `http://127.0.0.1:8880` by default. The buttons work without it; KDE just won't show a title or play state.

## 2. Find foobar2000's Wine prefix

Start foobar2000 the way you usually do, then run:

```bash
for p in $(pgrep -f foobar2000.exe); do tr '\0' '\n' < /proc/$p/environ | grep ^WINEPREFIX; done
```

If this prints nothing, foobar2000 uses the default prefix, `~/.wine`.

## 3. Install

```bash
./install.sh /path/to/media-control-bridge
```

The script does four things:
- copies the binary and `foobar2000-bridge-serve` to `~/.local/bin`
- installs two `systemd --user` services
- enables both services, so they start at login
- creates `~/.config/media-control-bridge/foobar2000.env`

Edit that `foobar2000.env` file: set `WINEPREFIX`, set `FOOBAR_EXE` to the full Linux path of `foobar2000.exe`, and set `WINE` if foobar2000 runs under Proton, Lutris or Bottles. Then start the services:

```bash
systemctl --user start foobar2000-bridge foobar2000-mpris
```

## 4. Check it

```bash
media-control-bridge send --connect 127.0.0.1:17777 --command status
```

If everything works, this prints the current title and play state, and the "foobar2000" player in KDE's media widget shows the same. To see why something isn't working, check the logs:

```bash
journalctl --user -u foobar2000-bridge -u foobar2000-mpris -f
```

## Notes

- Each button press starts a short `wine … /command` process, so expect a small delay.
- If foobar2000 isn't running, a button press starts it. While it's closed, KDE shows the player as stopped.
- After editing `foobar2000.env`, restart the server: `systemctl --user restart foobar2000-bridge`.
- To remove it: `systemctl --user disable --now foobar2000-bridge foobar2000-mpris`, then delete the installed files.
