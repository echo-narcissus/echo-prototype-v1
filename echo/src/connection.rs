use crate::protocol::{self, Operation};
use crate::store::SharedMessageStore;
use mio::event::Event;
use mio::net::TcpStream;
use mio::{Interest, Poll, Token};
use rand::{Rng, RngCore};
use rustls::{ServerConfig, ServerConnection};
use std::io::{self, Read, Write};
use std::sync::Arc;

pub struct Connection {
    verbose: bool,
    closing: bool,
    socket: TcpStream,
    token: Token,
    tls_conn: ServerConnection,
    message_store: SharedMessageStore, // the silly type defined in store.rs
    msg_id_size: usize,
    
    incoming_plaintext_buffer: Vec<u8>, // plaintext does not mean decrypting the actual user
    outgoing_plaintext_buffer: Vec<u8>, // message
  
    outgoing_tls_buffer: Vec<u8>,
}

impl Connection {
    pub fn new(
        socket: TcpStream,
        token: Token,
        tls_config: Arc<ServerConfig>,
        message_store: SharedMessageStore,
        msg_id_size: usize,
        verbose: bool
    ) -> io::Result<Self> {
        let tls_conn = ServerConnection::new(tls_config)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(Self {
            socket,
            token,
            tls_conn,
            message_store,
            msg_id_size,
            incoming_plaintext_buffer: Vec::new(),
            outgoing_plaintext_buffer: Vec::new(),
            outgoing_tls_buffer: Vec::new(),
            closing: false,
            verbose
        })
    }

    pub fn ready(&mut self, poll: &mut Poll, event: &Event) {
        if event.is_readable() {
            self.do_read();
        }

        if event.is_writable() {
            self.do_write();
        }

        if self.closing {
            // Connection is marked for closure, don't re-register.
            return;
        }

        // Process any decrypted plaintext.
        self.process_incoming_plaintext();

        // If we have data to send (either plaintext or encrypted), re-register.
        if !self.outgoing_plaintext_buffer.is_empty()
            || !self.outgoing_tls_buffer.is_empty()
        {
            self.encrypt_outgoing_plaintext();
        }

        self.reregister(poll);
    }

    // Read TLS data from the socket.
    fn do_read(&mut self) {
        loop {
            match self.tls_conn.read_tls(&mut self.socket) {
                Ok(0) => {
                    // Read EOF
                    self.closing = true;
                    return;
                }
                Ok(_) => {} // Keep going 
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Done reading for now
                    break;
                }
                Err(e) => {
                    // Real error
                    eprintln!("Read error on token {}: {}", self.token.0, e);
                    self.closing = true;
                    return;
                }
            }
        }

        if let Err(e) = self.tls_conn.process_new_packets() {
            eprintln!("TLS process error: {}", e);
            self.closing = true;
            return;
        }

        // Read any decrypted plaintext
        let mut plaintext_buf = [0; 4096];
        loop {
            match self.tls_conn.reader().read(&mut plaintext_buf) {
                Ok(0) => break, // No more plaintext
                Ok(n) => { // if there's more, make the buffer bigger
                    self.incoming_plaintext_buffer
                        .extend_from_slice(&plaintext_buf[..n]);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break; // No more plaintext for now
                }
                Err(e) => {
                    eprintln!("Plaintext read error: {}", e);
                    self.closing = true;
                    return;
                }
            }
        }
    }

    // Parse complete operations from the plaintext buffer
    fn process_incoming_plaintext(&mut self) {
        while let Some(op) = protocol::parse_message_from_buffer(
            &mut self.incoming_plaintext_buffer,
            self.msg_id_size,
            self.verbose
        ) {
            self.handle_operation(op);
        }
    }

    // Handle a parsed operation
    fn handle_operation(&mut self, op: Operation) {
        match op {
            Operation::Store { id, data, ttl } => {
                let mut store = self.message_store.lock().unwrap();
                store.insert(id, data, ttl);
                // Send a simple "OK" response: [0x00]
                self.outgoing_plaintext_buffer.push(0x00);
            }
            Operation::Retrieve { id } => {
                let mut store = self.message_store.lock().unwrap();
                match store.get(&id) {
                    Some(data) => {
                        // Found: LEN(u32) + DATA
                        self.outgoing_plaintext_buffer
                            .extend_from_slice(&(data.len() as u32).to_be_bytes());
                        self.outgoing_plaintext_buffer.extend_from_slice(&data);
                    }
                    None => {
                        let mut rng = rand::rng();
                        // Return a random block of length between 64 and 1024 bytes
                        let rand_len = rng.random_range(64..=1024) as u32;
                        let mut rand_data = vec![0; rand_len as usize];
                        rng.fill_bytes(&mut rand_data);
                        self.outgoing_plaintext_buffer
                            .extend_from_slice(&rand_len.to_be_bytes());
                        self.outgoing_plaintext_buffer.extend_from_slice(&rand_data);
                    }
                }
            }
        }
    }

    fn encrypt_outgoing_plaintext(&mut self) {
        if self.outgoing_plaintext_buffer.is_empty() {
            return;
        }

        let written = self
            .tls_conn
            .writer()
            .write(&self.outgoing_plaintext_buffer)
            .unwrap_or(0);

        self.outgoing_plaintext_buffer.drain(..written);
    }

    fn do_write(&mut self) {
        // First check if there is data to send
        while self.tls_conn.wants_write() {
            match self.tls_conn.write_tls(&mut self.outgoing_tls_buffer) {
                Ok(_) => {
                    // loop again in case there is more to write 
                }
                Err(e) => {
                    eprintln!("TLS write error: {}", e);
                    self.closing = true;
                    return;
                }
            }
        }

        // Try to write the buffered encrypted data
        if self.outgoing_tls_buffer.is_empty() {
            return;
        }

        loop {
            match self.socket.write(&self.outgoing_tls_buffer) {
                Ok(0) => {
                    // Should not happen, but not worth a panic
                    eprintln!("Write 0 bytes on token {}", self.token.0);
                    self.closing = true;
                    return;
                }
                Ok(n) => {
                    self.outgoing_tls_buffer.drain(..n);
                    if self.outgoing_tls_buffer.is_empty() {
                        break;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break; // Socket is full
                }
                Err(e) => {
                    eprintln!("Write error on token {}: {}", self.token.0, e);
                    self.closing = true;
                    return;
                }
            }
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closing
    }

    // Register with the poll
    pub fn register(&mut self, poll: &mut Poll) {
        poll.registry()
            .register(&mut self.socket, self.token, Interest::READABLE)
            .unwrap();
    }

    // Re-register with the poll (i forgot why this is neccesary but its in all the examples)
    fn reregister(&mut self, poll: &mut Poll) {
        let mut interest = Interest::READABLE;

        // If there is data to write, need to be notified when the socket is writable
        if !self.outgoing_plaintext_buffer.is_empty()
            || !self.outgoing_tls_buffer.is_empty()
            || self.tls_conn.wants_write()
        {
            interest |= Interest::WRITABLE;
        }

        poll.registry()
            .reregister(&mut self.socket, self.token, interest)
            .unwrap();
    }
}
