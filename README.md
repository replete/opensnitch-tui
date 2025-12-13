# OpenSnitch TUI

A Terminal UI control plane for [OpenSnitch](https://github.com/evilsocket/opensnitch), an interactive application firewall for Linux inspired by Little Snitch.

![TUI screenshot](static/screenshot.png)

This TUI is built in Rust, namely using the `tokio`, `tonic`, and `ratatui` crates. This program currently implements a subset of functions that the [OpenSnitch GUI](https://github.com/evilsocket/opensnitch/wiki/Getting-started) supports. Some features may never be supported due to complexity (e.g. support for multiple nodes).

## Features

This TUI tries to replace the official OpenSnitch GUI in single-node environments where it may be inconvenient/impossible to use the GUI.

* View high-level daemon runtime stats
* View trapped connection attempts that require a disposition (allow/deny)
* Easy keybindings to allow/deny trapped network flows
* View incoming alerts

The GUI may still be used separately (see below) for features the TUI doesn't yet support.

## Usage

The OpenSnitch daemon connects to a control plane server (like this TUI) to talk gRPC. This TUI now supports **both Unix domain sockets and TCP** for transport.

### Unix Domain Socket (Recommended)

OpenSnitch's [default config](https://github.com/evilsocket/opensnitch/wiki/Configurations) uses a Unix domain socket. To use the TUI with the default configuration:

```sh
# The TUI will bind to a Unix socket
$ opensnitch-tui --bind "unix:///tmp/osui.sock"
```

Make sure your OpenSnitch daemon config (`/etc/opensnitchd/default-config.json`) has the `Address` field set to the same Unix socket:
```json
{
    "Server": {
        "Address": "unix:///tmp/osui.sock"
    }
}
```

For better security, you can use the recommended per-user socket path:
```sh
$ opensnitch-tui --bind "unix:///run/user/1000/opensnitch/osui.sock"
```

### TCP Transport (Alternative)

Alternatively, you can use TCP transport. In the OpenSnitch daemon config (`/etc/opensnitchd/default-config.json`), change the `Address` field to a loopback-assigned IP address and port:
```sh
$ head -n4 /etc/opensnitchd/default-config.json
{
    "Server":
    {
        "Address":"127.0.0.1:50051",
```

Then run the TUI with:
```sh
$ opensnitch-tui --bind "127.0.0.1:50051"
```

Remember to update your invocation of the official GUI (`opensnitch-ui`) to pass the matching socket flag (e.g., `--socket "unix:///tmp/osui.sock"` or `--socket "127.0.0.1:50051"`).

**Note that only one of the GUI or TUI can run at one time, so kill the `opensnitch-ui` or `opensnitch-tui` process to run the other.**

### Pre-built Binaries

Download the latest release for your architecture:

```sh
# x86_64
curl -fsSL https://github.com/amalbansode/opensnitch-tui/releases/latest/download/opensnitch-tui-linux-x86_64 -o opensnitch-tui
chmod +x opensnitch-tui

# aarch64 (ARM64)
curl -fsSL https://github.com/amalbansode/opensnitch-tui/releases/latest/download/opensnitch-tui-linux-aarch64 -o opensnitch-tui
chmod +x opensnitch-tui
```

Or in a Dockerfile:

```dockerfile
# x86_64
RUN curl -fsSL https://github.com/amalbansode/opensnitch-tui/releases/latest/download/opensnitch-tui-linux-x86_64 -o /usr/local/bin/opensnitch-tui && \
    chmod +x /usr/local/bin/opensnitch-tui

# aarch64 (ARM64)
RUN curl -fsSL https://github.com/amalbansode/opensnitch-tui/releases/latest/download/opensnitch-tui-linux-aarch64 -o /usr/local/bin/opensnitch-tui && \
    chmod +x /usr/local/bin/opensnitch-tui
```

### Build from Source

```sh
$ cd $THIS_REPO
$ cargo build --release
$ cp target/release/opensnitch-tui $SOMEWHERE_IN_YOUR_PATH
$ opensnitch-tui --help
```

## Disclaimer

I'm pretty new to Rust and am using this project as an exercise to learn more. Use this software at your own risk. Contributions are welcome.
