use std::convert::TryInto;

const OP_STORE: u8 = 0x01;
const OP_RETRIEVE: u8 = 0x02;

const EXPIRY_FLAG_NONE: u8 = 0x00; // unused, to be reminded of for the protocol
const EXPIRY_FLAG_PRESENT: u8 = 0x01;

#[derive(Debug)]
pub enum Operation { // Enums of structs are so cool
    Store {
        id: Vec<u8>,
        data: Vec<u8>,
        ttl: Option<u64>,
    },
    Retrieve { 
        id: Vec<u8> 
    },
}

// Tries to parse one full operation from the start of the buffer.
// If successful, returns the operation and removes those bytes from the buffer.
// else, None
pub fn parse_message_from_buffer(
    buffer: &mut Vec<u8>,
    msg_id_size: usize,
    verbose: bool
) -> Option<Operation> {
    if buffer.is_empty() {
        return None;
    }

    let op_type = buffer[0];
    match op_type {
        OP_STORE => {
            if verbose {println!("Store Operation Recieved");}
            parse_store_op(buffer, msg_id_size)
        },
        OP_RETRIEVE => {
            if verbose {println!("Retrieve Operation Recieved");}
            parse_retrieve_op(buffer, msg_id_size)
        }
        _ => {
            // Invalid op_type, clear buffer to prevent looping
            // TODO: This could lead to dropping messages, which is not good
            eprintln!("Invalid operation type: {}", op_type);
            buffer.clear();
            None
        }
    }
}

fn parse_store_op(buffer: &mut Vec<u8>, msg_id_size: usize) -> Option<Operation> {
    // Header length = 1 (op) + N (msg id) + 4 (data length)
    let header_len = 1 + msg_id_size + 4;
    if buffer.len() < header_len {
        return None; // Not enough data for header (womp womp)
    }

    let len_bytes: [u8; 4] = buffer[1 + msg_id_size..header_len].try_into().unwrap();
    let data_len = u32::from_be_bytes(len_bytes) as usize;

    // Body length = data_len + 1 (expiry flag)
    let base_body_len = data_len + 1;
    if buffer.len() < header_len + base_body_len {
        return None;
    }

    let expiry_flag = buffer[header_len + data_len];
    let mut total_len = header_len + base_body_len;

    let ttl = if expiry_flag == EXPIRY_FLAG_PRESENT {
        // Optional TTL: 8 bytes
        let ttl_len = 8;
        if buffer.len() < total_len + ttl_len {
            return None; // Not enough data for TTL
        }
        total_len += ttl_len;
        let ttl_bytes: [u8; 8] = buffer
            [total_len - ttl_len..total_len]
            .try_into()
            .unwrap();
        Some(u64::from_be_bytes(ttl_bytes))
    } else {
        None
    };

    let op_bytes = buffer.drain(..total_len).collect::<Vec<u8>>(); // whoever came up with the
                                                                   // drain function is a genius

    let id = op_bytes[1..1 + msg_id_size].to_vec();
    let data = op_bytes[header_len..header_len + data_len].to_vec();

    Some(Operation::Store { id, data, ttl })
}

fn parse_retrieve_op(buffer: &mut Vec<u8>, msg_id_size: usize) -> Option<Operation> {
    // Message length = 1(op) + n (msg id)
    let total_len = 1 + msg_id_size;
    if buffer.len() < total_len {
        return None;
    }
    let mut op_bytes = buffer.drain(..total_len).collect::<Vec<u8>>();
    let id = op_bytes.split_off(1);
    Some(Operation::Retrieve { id }) 
}
