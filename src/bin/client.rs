use anyhow::{bail, Context, Result};
use lazy_static::lazy_static;
use secstr::SecUtf8;
use serde::{Deserialize, Serialize};
use ssh_nicety_common::{Message, MessageRequest};
use std::env;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Default, Serialize)]
struct ClientConfig {
    socket: Option<std::path::PathBuf>,
    secret: Option<SecUtf8>,
}

lazy_static! {
    static ref NAME: String = format!("{}-{}", env!("CARGO_PKG_NAME"), env!("CARGO_BIN_NAME"));
}

fn main() -> Result<()> {
    println!(
        "Config file location: {:?}",
        confy::get_configuration_file_path(&NAME, None).context("Unable to compute config path")?
    );
    let cfg: ClientConfig = confy::load(&NAME, None).context("Unable to load config")?;
    let socket_path = match (cfg.socket, std::env::var_os("XDG_RUNTIME_DIR")) {
        (Some(path), _) => path,
        (None, Some(var)) => {
            let mut p = PathBuf::from(var);
            p.push("ssh-nicety-client.socket");
            p
        }
        (None, None) => bail!("Unable to determine location for socket"),
    };
    println!("Using socket-path {:?}", socket_path);
    let secret = cfg
        .secret
        .context("Please specify a secret in the config")?;

    let request = match env::args().next().as_deref() {
        Some("code") => MessageRequest::Code {
            path: absolute_path(env::args().nth(1))?,
        },
        Some("terminal") => MessageRequest::Terminal {
            path: absolute_path(env::args().nth(1))?,
        },

        _ => bail!("Unrecognized executable"),
    };

    let message = Message { secret, request };

    let stream = UnixStream::connect(socket_path).context("Unable to connect to socket")?;
    serde_json::to_writer(stream, &message).context("Unable to write message")?;

    serde_json::to_writer_pretty(std::io::stdout(), &message.request)
        .context("Unable to print message")?;
    println!();
    Ok(())
}

fn absolute_path(path: Option<String>) -> Result<String> {
    let mut file = std::env::current_dir().context("Couldn't get current directory")?;
    if let Some(path) = path {
        file.push(path);
    };
    Ok(file
        .to_str()
        .context("Unable to get path as unicode")?
        .into())
}
