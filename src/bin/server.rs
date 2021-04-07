use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use listenfd::ListenFd;
use secstr::SecUtf8;
use serde::{Deserialize, Serialize};
use ssh_nicety_common::{Message, MessageRequest};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Command;

#[derive(Deserialize, Eq, PartialEq, Debug, Serialize)]
struct RemoteConfig {
    ssh_name: String,
    secret: SecUtf8,
}

#[derive(Deserialize, Debug, Default, Serialize)]
struct ServerConfig {
    socket: Option<std::path::PathBuf>,
    remotes: Vec<RemoteConfig>,
}

lazy_static! {
    static ref NAME: String = format!("{}-{}", env!("CARGO_PKG_NAME"), env!("CARGO_BIN_NAME"));
}

fn main() -> Result<()> {
    println!(
        "Config file location: {:?}",
        confy::get_configuration_file_path(&NAME, None).context("Unable to compute config path")?
    );
    let cfg: ServerConfig = confy::load(&NAME, None).context("Unable to load config")?;
    let listener = match (ListenFd::from_env().take_unix_listener(0), cfg.socket) {
        (Ok(Some(l)), _) => l,
        (Err(err), _) => Err(err).context("Unable to take unix socket")?,
        (Ok(None), Some(path)) => UnixListener::bind(path).context("Unable to make unix socket")?,
        (Ok(None), None) => bail!("Did not receive socket Listener or path"),
    };
    match listener.local_addr() {
        Ok(local) => println!("Listening on {:?}", local),
        Err(e) => eprintln!("Unable to determine local address: {:?}", e),
    }
    for stream in listener.incoming() {
        match stream {
            Err(e) => eprintln!("Incoming connection failed {:?}", e),
            Ok(stream) => {
                if let Err(e) = accept(&stream) {
                    eprintln!("{:?}", e);
                }
            }
        }
    }
    Ok(())
}

fn accept(stream: &UnixStream) -> Result<()> {
    let cfg: ServerConfig = confy::load(&NAME, None).context("Unable to load config")?;

    let Message { secret, request } =
        serde_json::from_reader::<_, Message>(stream).context("Unable to parse message")?;
    let remote = cfg
        .remotes
        .iter()
        .find(|remote| remote.secret == secret)
        .context("Unable to find remote with matching secret")?;

    let mut command = match request {
        MessageRequest::Terminal { path } => {
            let mut c = Command::new("gnome-terminal");
            c.args(&[
                "--tab",
                "--",
                "ssh",
                "-t",
                &remote.ssh_name,
                &format!(
                    "cd {}; exec $SHELL -l",
                    shell_escape::unix::escape(path.into())
                ),
            ]);
            c
        }
        MessageRequest::Code { path } => {
            let mut c = Command::new("code");
            c.args(&[
                "--folder-uri",
                &format!("vscode-remote://ssh-remote+{}{}", remote.ssh_name, path),
            ]);
            c
        }
    };

    println!("{:?}", command);
    let status = command.status().context("Unable to run command")?;
    println!("Exit status: {}", status);
    Ok(())
}
