use crate::connection::Connection;
use crate::store::SharedMessageStore;
use mio::event::Event;
use mio::net::TcpListener;
use mio::{Events, Interest, Poll, Token};
use rustls::ServerConfig;
use slab::Slab;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

const SERVER_TOKEN: Token = Token(usize::MAX);

pub(crate) struct TlsServer {
    server_socket: TcpListener,
    poll: Poll,
    connections: Slab<Connection>,
    tls_config: Arc<ServerConfig>,
    message_store: SharedMessageStore,
    msg_id_size: usize,
}

impl TlsServer {
    pub(crate) fn new(
        addr: SocketAddr,
        tls_config: Arc<ServerConfig>,
        message_store: SharedMessageStore,
        msg_id_size: usize,
    ) -> io::Result<Self> {
        let mut server_socket = TcpListener::bind(addr)?;
        let poll = Poll::new()?;

        poll.registry().register(
            &mut server_socket,
            SERVER_TOKEN,
            Interest::READABLE,
        )?;

        Ok(Self {
            server_socket,
            poll,
            connections: Slab::with_capacity(1024),
            tls_config,
            message_store,
            msg_id_size,
        })
    }
 
    // Main server event loop
    pub(crate) fn run(&mut self, verbose: bool) -> io::Result<()> {
        let mut events = Events::with_capacity(1024);
        loop {
            self.poll.poll(&mut events, Some(Duration::from_secs(1)))?;

            for event in &events {
                match event.token() {
                    SERVER_TOKEN => {
                        // New connections
                        self.accept_connections(verbose)?;
                    }
                    token => {
                        // Event on existing connection
                        self.handle_connection_event(token, event)?;
                    }
                }
            }
        }
    }

    // Accept pending connections
    fn accept_connections(&mut self, verbose: bool) -> io::Result<()> {
        loop {
            match self.server_socket.accept() {
                Ok((socket, addr)) => {
                    println!("Accepted new connection from: {}", addr);

                    if self.connections.len() >= self.connections.capacity() {
                        eprintln!("Connection slab is full, dropping connection");
                        // Dropping the socket will close it.
                        continue;
                    }

                    let entry = self.connections.vacant_entry();
                    let token = Token(entry.key());

                    let connection = Connection::new(
                        socket,
                        token,
                        self.tls_config.clone(),
                        self.message_store.clone(),
                        self.msg_id_size,
                        verbose
                    )?;

                    entry.insert(connection).register(&mut self.poll);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // No more connections pending
                    break;
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    // Handle an event for a specific connection token.
    fn handle_connection_event(&mut self, token: Token, event: &Event) -> io::Result<()> {
        if let Some(conn) = self.connections.get_mut(token.0) {
            conn.ready(&mut self.poll, event);

            if conn.is_closed() {
                // Connection closed, remove it.
                println!("Connection closed for token: {}", token.0);
                self.connections.remove(token.0);
            }
        } else {
            // token not in slab
        }
        Ok(())
    }
}
