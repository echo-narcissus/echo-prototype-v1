use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    // Path to a TLS certificate file in the PEM format
    #[arg(short, long, required = true)]
    pub(crate) cert: PathBuf,

    // Path to a TLS private key file in the PEM format
    #[arg(short, long, required = true)]
    pub(crate) key: PathBuf,

    // Port to listen on - Default of 4433
    #[arg(short, long, default_value_t = 4433)]
    pub(crate) port: u16,

    // IP Address to bind to
    #[arg(short, long, default_value = "127.0.0.1")]
    pub(crate) bind_addr: String,

    // Size of the message ID in bytes (512 / 8 = 64)
    #[arg(short, long, default_value_t = 64)]
    pub(crate) msg_id_size: usize,
}

impl Cli {
    pub(crate) fn parse_args() -> Self {
        Self::parse()
    }
}
