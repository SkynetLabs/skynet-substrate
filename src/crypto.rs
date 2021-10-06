use crate::encoding;

use bytes::{BufMut, BytesMut};
use ed25519_dalek;
use sp_core::hashing;

pub type Signature = [u8; SIGNATURE_LENGTH];

pub const HASH_LENGTH: usize = 32;

pub const PUBLIC_KEY_LENGTH: usize = ed25519_dalek::PUBLIC_KEY_LENGTH * 2;

pub const PRIVATE_KEY_LENGTH: usize = ed25519_dalek::SECRET_KEY_LENGTH * 2;

pub const SIGNATURE_LENGTH: usize = ed25519_dalek::SIGNATURE_LENGTH;

pub fn hash_data_key(data_key: &str) -> Vec<u8> {
    let bytes = encoding::encode_str(data_key);
    hashing::blake2_256(&bytes).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::str_to_bytes;
    use crate::encoding::encode_bytes_to_hex_bytes;

    #[test]
    fn should_hash_data_keys() {
        let hash = hash_data_key("");
        assert_eq!(
            encode_bytes_to_hex_bytes(&hash),
            str_to_bytes("81e47a19e6b29b0a65b9591762ce5143ed30d0261e5d24a3201752506b20f15c")
        );

        let hash = hash_data_key("skynet");
        assert_eq!(
            encode_bytes_to_hex_bytes(&hash),
            str_to_bytes("31c7a4d53ef7bb4c7531181645a0037b9e75c8b1d1285b468ad58bad6262c777")
        )
    }
}
