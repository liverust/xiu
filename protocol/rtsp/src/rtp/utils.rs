use super::define;
use super::errors::PackerError;
use super::errors::UnPackerError;
use bytesio::bytes_reader::BytesReader;

use bytes::{BufMut, BytesMut};

pub trait Unmarshal<T1, T2> {
    fn unmarshal(data: T1) -> T2
    where
        Self: Sized;
}

pub trait Marshal<T> {
    fn marshal(&self) -> T;
}

pub trait TPacker {
    fn pack(&mut self, nalus: &mut BytesMut) -> Result<(), PackerError>;
}
pub trait TRtpPacker: TPacker {
    fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError>;
}

pub trait TUnPacker {
    fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError>;
}

pub(super) fn is_fu_start(fu_header: u8) -> bool {
    fu_header & define::FU_START > 0
}

pub(super) fn is_fu_end(fu_header: u8) -> bool {
    fu_header & define::FU_END > 0
}

pub fn find_start_code(nalus: &[u8]) -> Option<usize> {
    let pattern = [0x00, 0x00, 0x01];
    nalus.windows(pattern.len()).position(|w| w == pattern)
}

pub fn split_annexb_and_process<T: TRtpPacker>(
    nalus: &mut BytesMut,
    packer: &mut T,
) -> Result<(), PackerError> {
    while nalus.len() > 0 {
        /* 0x02,...,0x00,0x00,0x01,0x02..,0x00,0x00,0x01  */
        /*  |         |              |      |             */
        /*  -----------              --------             */
        /*   first_pos         distance_to_first_pos      */
        if let Some(first_pos) = find_start_code(&nalus[..]) {
            let mut nalu_with_start_code =
                if let Some(distance_to_first_pos) = find_start_code(&nalus[first_pos + 3..]) {
                    let mut second_pos = first_pos + 3 + distance_to_first_pos;
                    while second_pos > 0 && nalus[second_pos - 1] == 0 {
                        second_pos -= 1;
                    }
                    nalus.split_to(second_pos)
                } else {
                    nalus.split_to(nalus.len())
                };

            let nalu = nalu_with_start_code.split_off(first_pos + 3);
            return packer.pack_nalu(nalu);
        } else {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

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
            /* 0x02,...,0x00,0x00,0x01,0x02..,0x00,0x00,0x01  */
            /*  |         |              |      |             */
            /*  -----------              --------             */
            /*   first_pos              second_pos            */
            if let Some(first_pos) = find_start_code(&nalus[..]) {
                let mut nalu_with_start_code =
                    if let Some(distance_to_first_pos) = find_start_code(&nalus[first_pos + 3..]) {
                        let mut second_pos = first_pos + 3 + distance_to_first_pos;
                        println!("left: {} right: {}", first_pos, distance_to_first_pos);
                        while second_pos > 0 && nalus[second_pos - 1] == 0 {
                            second_pos -= 1;
                        }
                        // while nalus[pos_right ]
                        nalus.split_to(second_pos)
                    } else {
                        nalus.split_to(nalus.len())
                    };

                println!("nalu_with_start_code: {:?}", nalu_with_start_code.to_vec());

                let nalu = nalu_with_start_code.split_off(first_pos + 3);
                println!("nalu: {:?}", nalu.to_vec());
            } else {
                break;
            }
        }
    }
}
