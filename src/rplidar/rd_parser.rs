//! Parser State Machine

use std::mem;

#[derive(Clone, Debug, Default)]
pub struct ResponseDescriptorParser {
    state: ParserState,
    curr: FlatResponse,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlatResponse {
    pub payload_len: u32,
    pub send_mode: u8,
    pub dtype: u8,
}

impl ResponseDescriptorParser {
    pub fn feed(&mut self, byte: u8) -> Option<FlatResponse> {
        let mut res = None;

        match (self.state, byte) {
            (ParserState::WaitingForHeader, 0xA5) => self.state = ParserState::WaitingForHeaderInv,
            (ParserState::WaitingForHeaderInv, 0x5A) => {
                self.state = ParserState::ReadingDataLenSendMode(0)
            }

            (ParserState::ReadingDataLenSendMode(step), b) => {
                self.curr.payload_len |= (b as u32) << ((step) * 8);
                if step == 3 {
                    self.curr.send_mode = (b >> 6) & 3;
                    self.curr.payload_len &= 0x3FFFFFFF;
                    self.state = ParserState::ReadingDataType
                } else {
                    self.state = ParserState::ReadingDataLenSendMode(step + 1)
                }
            }

            (ParserState::ReadingDataType, d) => {
                self.curr.dtype = d;
                res = Some(mem::replace(&mut self.curr, FlatResponse::default()));
                self.state = ParserState::WaitingForHeader;
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
}

#[cfg(test)]
mod tests {
    use crate::rplidar::rd_parser::{FlatResponse, ResponseDescriptorParser};

    #[test]
    fn simple_response_descriptor() {
        let scan_resp = [0xA5, 0x5A, 0x05, 0x00, 0x00, 0x40, 0x81];

        let expected = FlatResponse {
            dtype: 0x81,
            send_mode: 0x01,
            payload_len: 5,
        };

        let mut parser = ResponseDescriptorParser::default();

        let mut rd = None;
        for byte in scan_resp {
            rd = parser.feed(byte);
            if rd.is_some() {
                break;
            }
        }

        assert_eq!(rd, Some(expected))
    }
}
