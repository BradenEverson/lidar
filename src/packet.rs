//! Packet Format

pub const START_FLAG: u8 = 0xA5;

pub const MAX_REQ_PACKET_SIZE: usize = 1 + 1 + 1 + 255 + 1;
static mut BUFFER: [u8; MAX_REQ_PACKET_SIZE] = [START_FLAG; MAX_REQ_PACKET_SIZE];

pub struct Packet {
    pub op: OpCode,
    pub len: u8,
    pub payload: [u8; 255],
}

impl Packet {
    pub fn new(op: OpCode, payload: &[u8]) -> Self {
        let mut cpy = [0; 255];
        cpy[0..payload.len()].copy_from_slice(payload);
        Self {
            op,
            len: payload.len() as u8,
            payload: cpy,
        }
    }
    pub fn to_bytes(self) -> &'static [u8] {
        let len = self.len as usize;

        // SAFETY : As described in the RPLIDAR datasheet, only
        // 1 request should ever be sent at a time, so we only
        // should even convert and send a single binary packet
        // at a time as well
        unsafe {
            println!("{:?}", self.op);
            BUFFER[1] = self.op as u8;

            if self.len != 0 {
                BUFFER[2] = self.len;
                BUFFER[3..3 + len].copy_from_slice(&self.payload[0..len]);

                let mut checksum = 0;

                checksum ^= START_FLAG;
                checksum ^= self.len;
                for i in 0..len {
                    checksum ^= self.payload[i];
                }

                BUFFER[len + 3] = checksum;

                &BUFFER[0..len + 4]
            } else {
                &BUFFER[0..2]
            }
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    Stop = 0x25,
    Reset = 0x40,
    Scan = 0x20,
    ExpressScan = 0x82,
    ForceScan = 0x21,
    GetInfo = 0x50,
    GetHealth = 0x52,
    GetSampleRate = 0x59,
    GetLidarConf = 0x84,
}
