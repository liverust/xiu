use bytes::{BufMut, BytesMut};

pub fn find_start_code(nalus: &[u8]) -> Option<usize> {
    let pattern = [0x00, 0x00, 0x01];
    nalus.windows(pattern.len()).position(|w| w == pattern)
}

fn split_and_process_annexb(nalus: &BytesMut) -> Option<usize> {
    let pattern = [0x00, 0x00, 0x01];
    nalus.windows(pattern.len()).position(|w| w == pattern)
}

#[cfg(test)]
mod tests {

    use super::split_and_process_annexb;
    use bytes::{BufMut, BytesMut};
    use indexmap::IndexMap;
    use std::io::{BufRead, BufReader, Read};

    fn find_start_code(nalus: &[u8]) -> Option<usize> {
        let pattern = [0x00, 0x00, 0x01];
        nalus.windows(pattern.len()).position(|w| w == pattern)
    }

    #[test]
    pub fn test_annexb_split() {
        let mut nalus = BytesMut::new();
        nalus.extend_from_slice(&[
            0x00, 0x00, 0x01, 0x02, 0x03, 0x05, 0x06, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04,
            0x00, 0x00, 0x01, 0x02, 0x03,
        ]);

        while nalus.len() > 0 {
            if let Some(pos_left) = find_start_code(&nalus[..]) {
                let mut nalu_with_start_code =
                    if let Some(mut pos_right) = find_start_code(&nalus[pos_left + 3..]) {
                        println!("left: {} right: {}", pos_left, pos_right);
                        while pos_right > 0 && nalus[pos_left + pos_right + 3 - 1] == 0 {
                            pos_right -= 1;
                        }
                        // while nalus[pos_right ]
                        nalus.split_to(pos_left + pos_right + 3)
                    } else {
                        nalus.split_to(nalus.len())
                    };

                println!("nalu_with_start_code: {:?}", nalu_with_start_code.to_vec());

                let nalu = nalu_with_start_code.split_off(pos_left + 3);
                println!("nalu: {:?}", nalu.to_vec());
            } else {
                break;
            }
        }
    }
}
