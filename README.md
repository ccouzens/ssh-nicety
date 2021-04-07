# ssh-nicety

Open new terminal tabs and VSCode from an ssh connection.

Some people use Tmux and vim when connected to a remote server. This program is
not for them.

## Background

When I write code locally, I will usually open a terminal and navigate to the
directory containing my project. I will then type `code .` to open the directory
in VSCode. I create multiple terminals by pressing `ctrl + shift + t` to perform
various actions simultaneously. These new tabs all start up in the same
directory.

I am trying to write more code on remote computers (or local VMs) for security
reasons. I can connect my terminal to the remote computer using `ssh`. Using
VSCode's
[remote-ssh](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-ssh)
extension I can edit remote files.

Unfortunately, this isn't a good experience. When using SSH, I cannot easily
open an editor to the current remote directory, or create a new terminal tab of
the same remote directory.

This project aims to fix this.

## Requirements

The setup guide assumes systemd on your local computer. But it is not required.

To open VSCode on the local computer, VSCode must be installed.

The terminal opened is always gnome-terminal. This could be made configurable to
support other terminals (including Mac ones like terminal.app and iterm2). If
there is interest in being able to open other terminals, please raise a Github
issue.

The 2 halves communicate using a Unix domain socket. This may rule out Windows.

## Setup

Build the server and client components of this project. If you are using
different operating systems on your local and remotes you may wish to build the
relevant components in the relevant environments; or cross compile.

```bash
# For the local computer
cargo build --release --bin server

# For the remote computer
cargo build --release --bin client
```

Copy the server to your path in the local computer. Copy the client to your path
in the remote computer.

```bash
# On the local computer
cp target/release/server ~/.local/bin/ssh-nicety-server

# On the remote computer
cp target/release/client ~/.local/bin/code
cp target/release/client ~/.local/bin/terminal
```

Create an ssh config (`man 5 ssh_config`, `~/.ssh/config`), to connect to the
remote computers. Substitute the first `/run/user/1000` with the remote value of
`"$XDG_RUNTIME_DIR"` and the second `/run/user/1000` with the local value of
`"$XDG_RUNTIME_DIR"`

```
# ~/.ssh/config on the local computer
Host vagrant-dev-vm
  HostName 192.168.122.206
  User vagrant
  RemoteForward "/run/user/1000/ssh-nicety-client.socket" "/run/user/1000/ssh-nicety-server.socket"
```

Create a local configuration file at
`~/.config/ssh-nicety-server/default-config.toml` containing a list of remotes.
For each remote, include the name from the `.ssh/config` file, and a secret. The
secret should be unique to the remote. Optionally, include a socket path.

```toml
# Socket path is optional
socket = "/run/user/1000/ssh-nicety-server.socket"

[[remotes]]
ssh_name = "vagrant-dev-vm"
secret = "my secret"

[[remotes]]
ssh_name = "dev-container"
secret = "my super secret"
```

Create remote configuration files at
`~/.config/ssh-nicety-client/default-config.toml` containing a secret and
optionally a socket path.

```toml
# Socket path is optional
socket = "/run/user/1000/ssh-nicety-client.socket"
secret = "my secret"
```

Create a systemd unit file on the local computer at
`~/.config/systemd/user/ssh-nicety.service`.

```ini
[Unit]
Description=SSH nicety server
Requires=ssh-nicety.socket

[Service]
ExecStart=%h/.local/bin/ssh-nicety-server
```

Create a systemd socket unit file on the local computer at
`~/.config/systemd/user/ssh-nicety.socket`.

```ini
[Unit]
Description=SSH nicety server socket

[Socket]
ListenStream=%t/ssh-nicety-server.socket

[Install]
WantedBy=default.target
```

Enable and start the socket unit

```bash
systemctl --user daemon-reload
systemctl --user enable --now ssh-nicety.socket
```

Install the remote ssh extension in VSCode

```bash
code --install-extension ms-vscode-remote.remote-ssh
```

## Security

Giving remote servers which might be hostile the ability to run programs on your
local computer should only be done with caution.

The following steps are taken to reduce the risk:

- Only a limited number of programs (gnome-terminal and VSCode) can be launched.
- The options that these can be launched with are safe.
  - Gnome-terminal will launch with an ssh client that `cd`s to a specific
    directory on the remote.
  - VSCode will launch opened to the remote directory.
- By using shared secrets, remotes can only open tooling to themselves.
- By using sockets, other users, networked devices and web pages are all unable
  to send messages.

The following steps are being considered:

- Set a short sleep after every message, to prevent DOS attacks from a hostile
  remote, and to prevent brute force shared secret cracking.

If you think these steps can be broken, or are insufficient, please let me know.
Unless, this becomes very successful, it is ok to put details in a Github issue.

## Protocol

1. The client opens a connection to the server using the socket.
2. The client sends a JSON message

```bash
nc -U "$XDG_RUNTIME_DIR/ssh-nicety-client.socket" <<< '
{
  "secret": "my secret",
  "type": "terminal",
  "path": "/home/vagrant"
}
'
```
