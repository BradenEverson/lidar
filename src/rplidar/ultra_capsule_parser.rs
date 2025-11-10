use std::mem::MaybeUninit;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy)]
pub struct UltraCapsuleResponse {
    pub start_angle_sync_q6: u16,
    pub ultra_cabins: [UltraCabin; 32],
    pub checksum_1: u8,
    pub checksum_2: u8,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UltraCabin {
    pub combined_x3: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct HQNode {
    pub flag: u8,
    pub quality: u16,
    pub angle_z_q14: u16,
    pub dist_mm_q2: u32,
}

pub struct UltraCapsuleParser {
    cached_scan_node_buf: [u8; 80],
    cached_scan_node_buf_pos: usize,
    is_previous_capsuledata_ready: bool,
    cached_previous_ultracapsuledata: Option<UltraCapsuleResponse>,
    cached_last_data_timestamp_us: u64,
}

impl UltraCapsuleParser {
    const SYNC_1: u8 = 0xA;
    const SYNC_2: u8 = 0x5;
    const SYNC_BIT: u16 = 0x8000;

    pub fn new() -> Self {
        Self {
            cached_scan_node_buf: [0u8; 80],
            cached_scan_node_buf_pos: 0,
            is_previous_capsuledata_ready: false,
            cached_previous_ultracapsuledata: None,
            cached_last_data_timestamp_us: 0,
        }
    }

    pub fn reset(&mut self) {
        self.cached_scan_node_buf_pos = 0;
        self.is_previous_capsuledata_ready = false;
        self.cached_previous_ultracapsuledata = None;
    }

    pub fn on_data(&mut self, data: &[u8]) -> Vec<HQNode> {
        let mut nodes = Vec::new();
        
        for &current_data in data {
            match self.cached_scan_node_buf_pos {
                0 => {
                    let tmp = current_data >> 4;
                    if tmp != Self::SYNC_1 {
                        self.is_previous_capsuledata_ready = false;
                        continue;
                    }
                }
                1 => {
                    let tmp = current_data >> 4;
                    if tmp != Self::SYNC_2 {
                        self.cached_scan_node_buf_pos = 0;
                        self.is_previous_capsuledata_ready = false;
                        continue;
                    }
                }
                79 => {
                    self.cached_scan_node_buf[79] = current_data;
                    self.cached_scan_node_buf_pos = 0;

                    if let Some(capsule) = self.parse_capsule() {
                        if self.verify_checksum(&capsule) {
                            let new_nodes = self.process_capsule_data(capsule);
                            nodes.extend(new_nodes);
                        } else {
                            self.is_previous_capsuledata_ready = false;
                        }
                    }
                    continue;
                }
                _ => {}
            }

            self.cached_scan_node_buf[self.cached_scan_node_buf_pos] = current_data;
            self.cached_scan_node_buf_pos += 1;
        }

        nodes
    }

    fn parse_capsule(&self) -> Option<UltraCapsuleResponse> {
        if self.cached_scan_node_buf_pos != 0 {
            return None;
        }

        let start_angle_sync_q6 = u16::from_le_bytes([
            self.cached_scan_node_buf[2],
            self.cached_scan_node_buf[3],
        ]);

        let mut ultra_cabins = [UltraCabin::default(); 32];
        
        for (i, cabin) in ultra_cabins.iter_mut().enumerate() {
            let offset = 4 + i * 4;
            cabin.combined_x3 = u32::from_le_bytes([
                self.cached_scan_node_buf[offset],
                self.cached_scan_node_buf[offset + 1],
                self.cached_scan_node_buf[offset + 2],
                self.cached_scan_node_buf[offset + 3],
            ]);
        }

        let checksum_1 = self.cached_scan_node_buf[0] & 0x0F;
        let checksum_2 = (self.cached_scan_node_buf[1] & 0x0F) as u8;

        Some(UltraCapsuleResponse {
            start_angle_sync_q6,
            ultra_cabins,
            checksum_1,
            checksum_2,
        })
    }

    fn verify_checksum(&self, capsule: &UltraCapsuleResponse) -> bool {
        let mut checksum: u8 = 0;
        
        for i in 2..self.cached_scan_node_buf.len() {
            checksum ^= self.cached_scan_node_buf[i];
        }

        let received_checksum = (capsule.checksum_1 & 0xF) | (capsule.checksum_2 << 4);
        checksum == received_checksum
    }

    fn process_capsule_data(&mut self, capsule: UltraCapsuleResponse) -> Vec<HQNode> {
        let current_timestamp = self.get_current_timestamp_us();
        let mut nodes = Vec::new();

        if self.is_previous_capsuledata_ready {
            if let Some(prev_capsule) = self.cached_previous_ultracapsuledata {
                let new_nodes = self.interpolate_nodes(&prev_capsule, &capsule, current_timestamp);
                nodes.extend(new_nodes);
            }
        }

        if capsule.start_angle_sync_q6 & Self::SYNC_BIT != 0 {
            if self.is_previous_capsuledata_ready {
                // Handle encoder reset/new scan
            }
            self.is_previous_capsuledata_ready = false;
        }

        self.cached_previous_ultracapsuledata = Some(capsule);
        self.is_previous_capsuledata_ready = true;
        self.cached_last_data_timestamp_us = current_timestamp;

        nodes
    }

    fn interpolate_nodes(
        &self,
        prev_capsule: &UltraCapsuleResponse,
        current_capsule: &UltraCapsuleResponse,
        current_timestamp: u64,
    ) -> Vec<HQNode> {
        let mut nodes = Vec::new();

        let current_start_angle_q8 = ((current_capsule.start_angle_sync_q6 & 0x7FFF) << 2);
        let prev_start_angle_q8 = ((prev_capsule.start_angle_sync_q6 & 0x7FFF) << 2);

        let mut diff_angle_q8 = current_start_angle_q8 as i32 - prev_start_angle_q8 as i32;
        if prev_start_angle_q8 > current_start_angle_q8 {
            diff_angle_q8 += 360 << 8;
        }

        let angle_inc_q16 = (diff_angle_q8 << 3) / 3;
        let mut current_angle_raw_q16 = (prev_start_angle_q8 << 8) as i32;

        for pos in 0..prev_capsule.ultra_cabins.len() {
            let dist_q2 = self.decode_ultra_cabin_distances(&prev_capsule.ultra_cabins, pos);
            
            for cpos in 0..3 {
                let sync_bit = if ((current_angle_raw_q16 + angle_inc_q16) % (360 << 16)) < angle_inc_q16 {
                    1
                } else {
                    0
                };

                let offset_angle_mean_q16 = self.calculate_offset_angle(dist_q2[cpos]);
                let angle_q6 = ((current_angle_raw_q16 - offset_angle_mean_q16) >> 10) as i32;
                
                let normalized_angle_q6 = self.normalize_angle(angle_q6);
                
                let hq_node = HQNode {
                    flag: sync_bit | ((1 - sync_bit) << 1),
                    quality: if dist_q2[cpos] != 0 { 0x2F << 2 } else { 0 },
                    angle_z_q14: ((normalized_angle_q6 << 8) / 90) as u16,
                    dist_mm_q2: dist_q2[cpos],
                };

                nodes.push(hq_node);
                current_angle_raw_q16 += angle_inc_q16;
            }
        }

        nodes
    }

    fn decode_ultra_cabin_distances(&self, cabins: &[UltraCabin], pos: usize) -> [u32; 3] {
        let combined_x3 = cabins[pos].combined_x3;
        let dist_major = (combined_x3 & 0xFFF) as i32;

        let dist_predict1 = ((combined_x3 as i32) << 10) >> 22;
        let dist_predict2 = (combined_x3 as i32) >> 22;

        let (dist_major_scaled, scale_lvl1) = self.varbitscale_decode(dist_major as u32);
        let (dist_major2_scaled, scale_lvl2) = if pos == cabins.len() - 1 {
            self.varbitscale_decode((cabins[0].combined_x3 & 0xFFF) as u32)
        } else {
            self.varbitscale_decode((cabins[pos + 1].combined_x3 & 0xFFF) as u32)
        };

        let dist_base1 = dist_major_scaled as i32;
        let dist_base2 = dist_major2_scaled as i32;

        let mut dist_q2 = [0u32; 3];
        
        dist_q2[0] = (dist_major_scaled << 2) as u32;
        
        if dist_predict1 != 0xFFFFFE00 && dist_predict1 != 0x1FF {
            let scaled_predict1 = dist_predict1 << scale_lvl1;
            dist_q2[1] = ((scaled_predict1 + dist_base1) << 2) as u32;
        }

        if dist_predict2 != 0xFFFFFE00 && dist_predict2 != 0x1FF {
            let scaled_predict2 = dist_predict2 << scale_lvl2;
            dist_q2[2] = ((scaled_predict2 + dist_base2) << 2) as u32;
        }

        dist_q2
    }

    fn varbitscale_decode(&self, scaled: u32) -> (u32, u32) {
        const VBS_SCALED_BASE: [u32; 5] = [
            0x400,
            0x800,
            0x1000,
            0x2000,
            0,
        ];

        const VBS_SCALED_LVL: [u32; 5] = [4, 3, 2, 1, 0];
        const VBS_TARGET_BASE: [u32; 5] = [
            0x1 << 10,
            0x1 << 11,
            0x1 << 12,
            0x1 << 13,
            0,
        ];

        for i in 0..VBS_SCALED_BASE.len() {
            let remain = scaled as i32 - VBS_SCALED_BASE[i] as i32;
            if remain >= 0 {
                return (VBS_TARGET_BASE[i] + (remain as u32 << VBS_SCALED_LVL[i]), VBS_SCALED_LVL[i]);
            }
        }

        (0, 0)
    }

    fn calculate_offset_angle(&self, dist_q2: u32) -> i32 {
        let mut offset_angle_mean_q16 = (7.5 * 3.1415926535 * (1 << 16) as f64 / 180.0) as i32;

        if dist_q2 >= (50 * 4) as u32 {
            let k1 = 98361;
            let k2 = (k1 / dist_q2 as i32).max(0);
            offset_angle_mean_q16 = (8.0 * 3.1415926535 * (1 << 16) as f64 / 180.0) as i32 
                - (k2 << 6) 
                - (k2 * k2 * k2) / 98304;
        }

        (offset_angle_mean_q16 as f64 * 180.0 / 3.14159265) as i32
    }

    fn normalize_angle(&self, angle_q6: i32) -> i32 {
        let mut angle = angle_q6;
        if angle < 0 {
            angle += 360 << 6;
        }
        if angle >= (360 << 6) {
            angle -= 360 << 6;
        }
        angle
    }

    fn get_current_timestamp_us(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }
}

impl Default for UltraCapsuleParser {
    fn default() -> Self {
        Self::new()
    }
}
