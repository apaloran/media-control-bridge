#!/bin/sh
# Installs the foobar2000 bridge for the current user and enables it at login.
# Usage: ./install.sh /path/to/media-control-bridge
set -eu
here=$(cd "$(dirname "$0")" && pwd)
bin=${1:?usage: $0 /path/to/media-control-bridge}

install -Dm755 "$bin" "$HOME/.local/bin/media-control-bridge"
install -Dm755 "$here/foobar2000-bridge-serve" "$HOME/.local/bin/foobar2000-bridge-serve"
install -Dm644 "$here/foobar2000-bridge.service" "$HOME/.config/systemd/user/foobar2000-bridge.service"
install -Dm644 "$here/foobar2000-mpris.service" "$HOME/.config/systemd/user/foobar2000-mpris.service"

cfg="$HOME/.config/media-control-bridge/foobar2000.env"
if [ ! -e "$cfg" ]; then
  install -Dm644 "$here/foobar2000.env.example" "$cfg"
  echo "Created $cfg - edit WINEPREFIX (and the other values) before starting."
fi

systemctl --user daemon-reload
systemctl --user enable foobar2000-bridge.service foobar2000-mpris.service
echo "Enabled. Start now with: systemctl --user start foobar2000-bridge foobar2000-mpris"
