//! Packet Format

pub const START_FLAG: u8 = 0xA5;

pub const MAX_REQ_PACKET_SIZE: usize = 1 + 1 + 1 + 255 + 1;
static mut BUFFER: [u8; MAX_REQ_PACKET_SIZE] = [START_FLAG; MAX_REQ_PACKET_SIZE];

pub struct Packet {
    pub command: Command,
    pub len: u8,
    pub payload: [u8; 255],
}

impl Packet {
    pub fn to_bytes(&self) -> &[u8] {
        let len = self.len as usize;

        // SAFETY : As described in the RPLIDAR datasheet, only
        // 1 request should ever be sent at a time, so we only
        // should even convert and send a single binary packet
        // at a time as well
        unsafe {
            BUFFER[1] = self.command.opcode();
            BUFFER[2] = self.len;

            BUFFER[3..3 + len].copy_from_slice(&self.payload[0..len]);

            &BUFFER[0..len + 4]
        }
    }
}

pub enum Command {}

impl Command {
    pub fn opcode(&self) -> u8 {
        todo!();
    }
}
