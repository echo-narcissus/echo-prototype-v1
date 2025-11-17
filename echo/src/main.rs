#![allow(dead_code)]

// Include Project Files
mod args;
mod config;
mod connection;
mod protocol; // TODO: TEMPORARY PROTOCOL
mod server;
mod store; // TODO: TEMPORARY MESSAGE STORAGE SOLUTION FOR TESTING
           // SHOULD BE REMOVED BEFORE REAL MESSAGES ARE SENT

// Third Party Imports
use std::{net::SocketAddr, sync::{Arc, Mutex}};

// First Party Imports
use crate::args::Cli;
use crate::server::TlsServer;

fn main() {

    let cli = Cli::parse_args();
    
    let tls_config = match config::load_tls_config(&cli.cert, &cli.key) {
        Ok(config) => Arc::new(config),
        Err(e) => {
            eprintln!("Failed to load TLS configuration.\n{}", e);
            std::process::exit(1);
        }
    };
     
    let store = Arc::new(Mutex::new(store::MessageStore::new())); // TODO TEMPORARY!!!!!

    let addr: SocketAddr = format!("{}:{}", cli.bind_addr, cli.port)
        .parse()
        .expect("Failed to parse bind address"); // dont need a period here cause it auto puts
                                                 // punctuation

    println!("Server starting on {}...", addr);

    //being able to use match statements in variable assignments is so fun
    let mut tls_server = match TlsServer::new(
        addr,
        tls_config,
        store,
        cli.msg_id_size,
    ) {
        Ok(server) => server,
        Err(e) => {
            eprintln!("Failed to start server: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = tls_server.run(cli.verbose) {
        eprintln!("Server runtime error: {}", e);
    }

}
