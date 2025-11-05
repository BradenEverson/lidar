//! Parser State Machine

use std::mem;

#[derive(Clone, Debug, Default)]
pub struct ResponseParser {
    state: ParserState,
    curr: FlatResponse,
}

#[derive(Clone, Debug, Default)]
pub struct FlatResponse {
    response_len: u32,
    send_mode: u8,
    dtype: u8,
    payload: Vec<u8>,
}

impl ResponseParser {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<FlatResponse> {
        let mut responses = vec![];
        for byte in bytes {
            if let Some(resp) = self.feed_single(*byte) {
                responses.push(resp);
            }
        }

        responses
    }

    pub fn feed_single(&mut self, byte: u8) -> Option<FlatResponse> {
        let mut res = None;

        match (self.state, byte) {
            (ParserState::WaitingForHeader, 0xA5) => self.state = ParserState::WaitingForHeaderInv,
            (ParserState::WaitingForHeaderInv, 0x5A) => {
                self.state = ParserState::ReadingDataLenSendMode(0)
            }

            (ParserState::ReadingDataLenSendMode(step), b) => {
                self.curr.response_len |= (b as u32) << ((3 - step) * 8);
                if step == 3 {
                    self.curr.send_mode = (self.curr.response_len & 3) as u8;
                    self.curr.response_len &= !3;
                    self.state = ParserState::ReadingDataType
                } else {
                    self.state = ParserState::ReadingDataLenSendMode(step + 1)
                }
            }

            (ParserState::ReadingDataType, d) => {
                self.curr.dtype = d;
            }

            (ParserState::ReadingPayload, p) => {
                self.curr.payload.push(p);
                if self.curr.payload.len() == self.curr.response_len as usize {
                    res = Some(mem::replace(&mut self.curr, FlatResponse::default()));
                    self.state = ParserState::WaitingForHeader;
                }
            }

            // Invalid state, revert to header
            _ => self.state = ParserState::WaitingForHeader,
        }

        res
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ParserState {
    #[default]
    WaitingForHeader,
    WaitingForHeaderInv,
    ReadingDataLenSendMode(u8),
    ReadingDataType,
    ReadingPayload,
}
