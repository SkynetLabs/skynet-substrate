use bytes::{BufMut, BytesMut};

// fn decode_hex_bytes_to_bytes(hex_bytes: &[u8]) -> Vec<u8> {}

pub fn encode_bytes_to_hex_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(u4_to_hex_byte((byte & 0xf0) >> 4));
        encoded.push(u4_to_hex_byte(byte & 0x0f));
    }
    encoded
}

fn u4_to_hex_byte(u4: u8) -> u8 {
    match u4 {
        n @ 0..=9 => 48 + n,
        n @ 10..=15 => 97 + n - 10,
        _ => panic!("Unexpected u4 input"),
    }
}

fn encode_number(mut num: usize) -> [u8; 8] {
    let mut encoded: [u8; 8] = [0; 8];
    for index in 0..encoded.len() {
        let byte = num & 0xff;
        encoded[index] = byte as u8;
        num >>= 8;
    }
    encoded
}

pub fn encode_str(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();

    let mut encoded = BytesMut::with_capacity(8 + bytes.len());
    let encoded_len = encode_number(bytes.len());
    encoded.put(encoded_len.as_ref());
    // Skip to position 8.
    for _ in 0..(8 - encoded_len.len()) {
        encoded.put_u8(0);
    }
    encoded.put(bytes);
    encoded.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::str_to_bytes;

    #[test]
    fn should_encode_hex() {
        let bytes = encode_bytes_to_hex_bytes(&[255]);
        assert_eq!(bytes, str_to_bytes("ff"));

        let bytes = encode_bytes_to_hex_bytes(&[10]);
        assert_eq!(bytes, str_to_bytes("0a"));

        let bytes = encode_bytes_to_hex_bytes(&[255, 10]);
        assert_eq!(bytes, str_to_bytes("ff0a"));
    }

    #[test]
    fn should_encode_number() {
        let bytes = encode_number(0);
        assert_eq!(bytes, [0, 0, 0, 0, 0, 0, 0, 0]);

        let bytes = encode_number(1);
        assert_eq!(bytes, [1, 0, 0, 0, 0, 0, 0, 0]);

        let bytes = encode_number(255);
        assert_eq!(bytes, [255, 0, 0, 0, 0, 0, 0, 0]);

        let bytes = encode_number(256);
        assert_eq!(bytes, [0, 1, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn should_encode_str() {
        let bytes = encode_str("");
        assert_eq!(bytes, vec![0, 0, 0, 0, 0, 0, 0, 0]);

        let bytes = encode_str("skynet");
        assert_eq!(
            bytes,
            vec![6, 0, 0, 0, 0, 0, 0, 0, 115, 107, 121, 110, 101, 116]
        );

        let bytes = encode_str("żźć");
        assert_eq!(
            bytes,
            vec![6, 0, 0, 0, 0, 0, 0, 0, 197, 188, 197, 186, 196, 135]
        );
    }
}
