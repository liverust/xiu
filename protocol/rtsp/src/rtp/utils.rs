use bytes::{BufMut, BytesMut};
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

    #[test]
    pub fn test_annexb_split() {
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&[
            0x00, 0x00, 0x01, 0x02, 0x03, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x01,
            0x02, 0x03,
        ]);

        if let Some(pos) = split_and_process_annexb(&bytes) {
            println!("annexb: {}", pos);
        };
    }
}
