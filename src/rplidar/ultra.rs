//! Ultra Capsule Parser

use crate::rplidar::response::UltraCapsuleResponse;

#[derive(Default)]
pub struct UltraCapsuleParser {
    previous: Option<UltraCapsuleResponse>,
}

impl UltraCapsuleParser {
    pub fn on_scan_node_capsule_data(
        &mut self,
        capsule: UltraCapsuleResponse,
    ) -> Option<ParsedUltraCapsule> {
        let mut res = None;

        if let Some(prev) = &self.previous {
            let curr_start_angle = ((capsule.start_angle_q6 & 0x7FFF) as i32) << 2;
            let prev_start_angle = ((prev.start_angle_q6 & 0x7FFF) as i32) << 2;

            let mut diff_angle_q8 = curr_start_angle - prev_start_angle;
            if prev_start_angle > curr_start_angle {
                diff_angle_q8 += 360 << 8;
            }

            let angle_inc_q16 = (diff_angle_q8 << 3) / 3;
            let mut curr_angle_raw_q16 = prev_start_angle << 8;

            let mut scans = Vec::with_capacity(96);

            for pos in 0..prev.ultra_cabins.len() {
                let combined_x3 = prev.ultra_cabins[pos];

                let dist_major = (combined_x3 & 0xFFF) as i32;
                let dist_predict1 = ((combined_x3 << 10) as i32) >> 22;
                let dist_predict2 = (combined_x3 as i32) >> 22;

                let dist_major2 = if pos == prev.ultra_cabins.len() - 1 {
                    (capsule.ultra_cabins[0] & 0xFFF) as i32
                } else {
                    (prev.ultra_cabins[pos + 1] & 0xFFF) as i32
                };

                let (dist_base1, scale_lvl1) = Self::varbitscale_decode(dist_major);
                let (dist_base2, scale_lvl2) = Self::varbitscale_decode(dist_major2);

                let mut dist_base1 = dist_base1;
                if dist_major == 0 && dist_major2 != 0 {
                    dist_base1 = dist_base2;
                }

                let dist_q2_0 = dist_major << 2;

                let dist_q2_1 = if dist_predict1 == 0xFFFFFE00u32 as i32 || dist_predict1 == 0x1FF {
                    0
                } else {
                    let dist_predict_scaled = dist_predict1 << scale_lvl1;
                    (dist_predict_scaled + dist_base1) << 2
                };

                let dist_q2_2 = if dist_predict2 == 0xFFFFFE00u32 as i32 || dist_predict2 == 0x1FF {
                    0
                } else {
                    let dist_predict_scaled = dist_predict2 << scale_lvl2;
                    (dist_predict_scaled + dist_base2) << 2
                };

                let distances = [dist_q2_0, dist_q2_1, dist_q2_2];

                for &dist_q2 in distances.iter() {
                    let sync_bit =
                        ((curr_angle_raw_q16 + angle_inc_q16) % (360 << 16)) < angle_inc_q16;

                    let mut offset_angle_mean_q16 =
                        (7.5 * 3.1415926535 * (1 << 16) as f64 / 180.0) as i32;

                    if dist_q2 >= (50 * 4) {
                        const K1: i32 = 98361;
                        let k2 = K1 / dist_q2;

                        offset_angle_mean_q16 = (8.0 * 3.1415926535 * (1 << 16) as f64 / 180.0)
                            as i32
                            - (k2 << 6)
                            - (k2 * k2 * k2) / 98304;
                    }

                    let angle_q6 = (curr_angle_raw_q16
                        - (offset_angle_mean_q16 as f32 * 180.0 / 3.1415926535) as i32)
                        >> 10;
                    curr_angle_raw_q16 += angle_inc_q16;

                    let angle_q6 = Self::normalize_angle(angle_q6);
                    let angle_z_q14 = (angle_q6 << 8) / 90;

                    scans.push(UltraCapsuleScan {
                        sync: sync_bit,
                        quality: if dist_q2 != 0 { 0x2F } else { 0 },
                        angle_z_q14,
                        dist_mm_q2: dist_q2,
                    });
                }
            }

            res = Some(ParsedUltraCapsule { scans });
        }

        self.previous = Some(capsule);
        res
    }

    fn varbitscale_decode(scaled: i32) -> (i32, u32) {
        const VBS_SCALED_BASE: [i32; 5] = [0x400, 0x200, 0x100, 0x80, 0];

        const VBS_SCALED_LVL: [u32; 5] = [4, 3, 2, 1, 0];
        const VBS_TARGET_BASE: [i32; 5] = [1 << 10, 1 << 9, 1 << 8, 1 << 7, 0];

        for i in 0..VBS_SCALED_BASE.len() {
            let remain = scaled - VBS_SCALED_BASE[i];
            if remain >= 0 {
                return (
                    VBS_TARGET_BASE[i] + (remain << VBS_SCALED_LVL[i]),
                    VBS_SCALED_LVL[i],
                );
            }
        }

        (0, 0)
    }

    fn normalize_angle(angle_q6: i32) -> i32 {
        let mut angle = angle_q6;
        if angle < 0 {
            angle += 360 << 6;
        }
        if angle >= (360 << 6) {
            angle -= 360 << 6;
        }
        angle
    }
}

pub struct ParsedUltraCapsule {
    pub scans: Vec<UltraCapsuleScan>,
}

pub struct UltraCapsuleScan {
    pub sync: bool,
    pub quality: u8,
    pub angle_z_q14: i32,
    pub dist_mm_q2: i32,
}

impl UltraCapsuleScan {
    pub fn angle_degrees(&self) -> f32 {
        (self.angle_z_q14 as f32) * 90.0 / (1 << 14) as f32
    }

    pub fn distance_mm(&self) -> f32 {
        (self.dist_mm_q2 as f32) / 4.0
    }
}
