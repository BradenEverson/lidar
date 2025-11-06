//! Payload parsing state machine

use std::mem;

#[derive(Debug, Clone, Default)]
pub struct PayloadParser {
    payload: Vec<u8>,
    len: usize,
}

impl PayloadParser {
    pub fn set_payload_len(&mut self, len: usize) {
        self.len = len;
        if self.payload.len() != 0 {
            self.payload = vec![];
        }
    }

    pub fn feed(&mut self, byte: u8) -> Option<Vec<u8>> {
        let mut res = None;
        self.payload.push(byte);
        if self.payload.len() == self.len {
            res = Some(mem::replace(&mut self.payload, vec![]));
        }

        res
    }
}
