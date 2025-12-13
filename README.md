# OpenSnitch TUI (Devcontainer Fork)

A Terminal UI control plane for [OpenSnitch](https://github.com/evilsocket/opensnitch), customized for devcontainer environments.

![TUI screenshot](static/screenshot.png)

This fork is optimized for use in devcontainers where you want simple hostname-based firewall rules. When you allow/deny a connection, it creates minimal rules matching just the destination hostname (using regexp to include subdomains), rather than complex rules matching user ID, process path, port, and protocol.

**Example rule created by this TUI:**
```json
{
  "name": "allow-registry.npmjs.org",
  "operator": {
    "type": "regexp",
    "operand": "dest.host",
    "data": "^(.*\\.)?registry\\.npmjs\\.org$"
  }
}
```

This is based on [amalbansode/opensnitch-tui](https://github.com/amalbansode/opensnitch-tui).

## Features

* View trapped connection attempts that require a disposition (allow/deny)
* Easy keybindings to allow/deny trapped network flows
* Creates simple hostname-based rules (regexp matching domain + subdomains)
* Falls back to IP-based rules when hostname is unavailable

## Usage

The OpenSnitch daemon connects to a control plane server (like this TUI) to talk gRPC.

### TCP Transport (Recommended)

In the OpenSnitch daemon config (`/etc/opensnitchd/default-config.json`), set the `Address` field to a loopback IP address and port:
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

Remember to update your invocation of the official GUI (`opensnitch-ui`) to pass the matching socket flag (e.g., `--socket "127.0.0.1:50051"`).

### Unix Domain Sockets (Not Fully Working)

While this TUI can bind to Unix domain sockets (e.g., `--bind "unix:///tmp/osui.sock"`), there is currently an **incompatibility between the Go gRPC client in opensnitchd and the Rust gRPC server (tonic/h2)**.

The issue: Go's gRPC library sends the Unix socket path as the HTTP/2 `:authority` header, but Rust's h2 library strictly validates this header per HTTP/2 spec and rejects socket paths as invalid URIs. This results in `PROTOCOL_ERROR` / `RST_STREAM` errors.

The official Python OpenSnitch UI works because Python's grpcio library is more permissive about authority header validation.

**Workaround options:**
1. Use TCP transport (recommended, as shown above)
2. Patch opensnitchd to add `grpc.WithAuthority("localhost")` when dialing Unix sockets
3. Wait for upstream fixes in either [h2](https://github.com/hyperium/h2/pull/487) or grpc-go

See [tonic#826](https://github.com/hyperium/tonic/issues/826) and [h2#487](https://github.com/hyperium/h2/pull/487) for more details.

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
