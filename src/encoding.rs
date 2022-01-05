use crate::crypto::{Signature, SIGNATURE_LENGTH};
use crate::skylink::BASE64_ENCODED_SKYLINK_SIZE;
use crate::util::str_to_bytes;

use bytes::{BufMut, BytesMut};
use sp_std::vec::Vec;

/// Encodes the bytes to a skylink encoded using base64 raw URL encoding.
pub fn encode_skylink_base64(bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    // Make sure we'll have a slice big enough.
    buf.resize(BASE64_ENCODED_SKYLINK_SIZE, 0);

    let _ = base64::encode_config_slice(bytes, base64::URL_SAFE_NO_PAD, &mut buf);
    buf
}

pub fn decode_hex_to_bytes(hex: &str) -> Vec<u8> {
    decode_hex_bytes_to_bytes(&str_to_bytes(hex))
}

pub fn decode_hex_bytes_to_bytes(hex_bytes: &[u8]) -> Vec<u8> {
    if hex_bytes.len() % 2 != 0 {
        panic!("Expected an even number of hex bytes");
    }

    let mut decoded = Vec::with_capacity(hex_bytes.len() / 2);
    for bytes in hex_bytes.chunks(2) {
        match bytes {
            [byte1, byte2] => {
                let byte = (hex_byte_to_u4(*byte1) << 4) | hex_byte_to_u4(*byte2);
                decoded.push(byte);
            }
            _ => panic!("Should not hit this branch"),
        }
    }
    decoded
}

pub fn encode_bytes_to_hex_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(u4_to_hex_byte((byte & 0xf0) >> 4));
        encoded.push(u4_to_hex_byte(byte & 0x0f));
    }
    encoded
}

fn hex_byte_to_u4(hex_byte: u8) -> u8 {
    match hex_byte {
        // 0-9
        n @ 48..=57 => n - 48,
        // a-f
        n @ 97..=102 => n - 97 + 10,
        _ => panic!("Unexpected hex_byte input"),
    }
}

fn u4_to_hex_byte(u4: u8) -> u8 {
    match u4 {
        // 0-9
        n @ 0..=9 => 48 + n,
        // a-f
        n @ 10..=15 => 97 + n - 10,
        _ => panic!("Unexpected u4 input"),
    }
}

pub fn encode_number(mut num: u64) -> [u8; 8] {
    let mut encoded: [u8; 8] = [0; 8];
    for encoded_byte in &mut encoded {
        let byte = num & 0xff;
        *encoded_byte = byte as u8;
        num >>= 8;
    }
    encoded
}

pub fn encode_prefixed_bytes(bytes: &[u8]) -> Vec<u8> {
    let len = bytes.len();
    let mut encoded = BytesMut::with_capacity(8 + len);

    encoded.put_u64_le(len as u64);
    encoded.put(bytes.as_ref());

    encoded.to_vec()
}

pub fn encode_str(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();

    let mut encoded = BytesMut::with_capacity(8 + bytes.len());
    let encoded_len = encode_number(bytes.len() as u64);

    encoded.put(encoded_len.as_ref());
    // Skip to position 8.
    for _ in 0..(8 - encoded_len.len()) {
        encoded.put_u8(0);
    }
    encoded.put(bytes);

    encoded.to_vec()
}

pub fn vec_to_signature(v: Vec<u8>) -> Signature {
    if v.len() != SIGNATURE_LENGTH {
        panic!("Input v is of the wrong signature length");
    }

    let mut signature = [0; SIGNATURE_LENGTH];
    for (i, byte) in v.into_iter().enumerate() {
        signature[i] = byte;
    }
    signature
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::str_to_bytes;

    use sp_std::vec;

    #[test]
    fn should_decode_hex() {
        let s = decode_hex_to_bytes("ff");
        assert_eq!(s, vec![255]);

        let s = decode_hex_to_bytes("0a");
        assert_eq!(s, vec![10]);

        let s = decode_hex_to_bytes("ff0a");
        assert_eq!(s, vec![255, 10]);
    }

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
