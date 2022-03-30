use crate::crypto::hash_all;
use crate::encoding::{
    decode_hex_to_bytes, decode_skylink_base64, encode_prefixed_bytes, encode_skylink_base64,
};
use crate::util::{str_to_bytes, trim_prefix, URI_SKYNET_PREFIX};

use bytes::{BufMut, BytesMut};
use sp_std::vec::Vec;

/// The string length of the Skylink after it has been encoded using base64.
pub const BASE64_ENCODED_SKYLINK_SIZE: usize = 46;

/// The raw size in bytes of the data that gets put into a link.
pub const RAW_SKYLINK_SIZE: usize = 34;

pub struct SiaSkylink {
    pub bitfield: u16,
    pub merkle_root: Vec<u8>,
}

impl SiaSkylink {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut encoded = BytesMut::with_capacity(RAW_SKYLINK_SIZE);

        encoded.put_u16_le(self.bitfield);
        encoded.put(self.merkle_root.as_ref());

        encoded.to_vec()
    }

    pub fn to_string(&self) -> Vec<u8> {
        encode_skylink_base64(&self.to_bytes())
    }
}

/// Creates a new Sia public key. Matches `Ed25519PublicKey` in Sia.
pub fn new_ed25519_public_key(public_key: &str) -> SiaPublicKey {
    let algorithm = new_specifier("ed25519");
    let public_key_bytes = decode_hex_to_bytes(public_key);

    SiaPublicKey {
        algorithm,
        key: public_key_bytes,
    }
}

/// Creates a new v2 skylink. Matches `NewSkylinkV2` in skyd.
pub fn new_skylink_v2(sia_public_key: SiaPublicKey, tweak: &[u8]) -> SiaSkylink {
    const VERSION: u16 = 2;

    let bitfield = VERSION - 1;
    let merkle_root = derive_registry_entry_id(sia_public_key, tweak);
    SiaSkylink {
        bitfield,
        merkle_root,
    }
}

const SPECIFIER_LEN: usize = 16;

/// A helper to derive an entry id for a registry key value pair. Matches `DeriveRegistryEntryID` in
/// Sia.
fn derive_registry_entry_id(public_key: SiaPublicKey, tweak: &[u8]) -> Vec<u8> {
    hash_all(&[&public_key.marshal_sia(), tweak])
}

/// Returns a specifier for given name, a specifier can only be 16 bytes so we panic if the given
/// name is too long.
fn new_specifier(name: &str) -> Vec<u8> {
    let mut encoded = BytesMut::with_capacity(SPECIFIER_LEN);

    let name_bytes = str_to_bytes(name);
    encoded.put(name_bytes.as_ref());
    for _ in 0..SPECIFIER_LEN - name_bytes.len() {
        encoded.put_u8(0);
    }

    encoded.to_vec()
}

const PUBLIC_KEY_SIZE: usize = 32;

pub struct SiaPublicKey {
    pub algorithm: Vec<u8>,
    pub key: Vec<u8>,
}

impl SiaPublicKey {
    fn marshal_sia(&self) -> Vec<u8> {
        let mut encoded = BytesMut::with_capacity(SPECIFIER_LEN + 8 + PUBLIC_KEY_SIZE);

        encoded.put(self.algorithm.as_ref());
        encoded.put(encode_prefixed_bytes(&self.key).as_ref());

        encoded.to_vec()
    }
}

pub fn decode_skylink(skylink: &str) -> Vec<u8> {
    let encoded = trim_prefix(skylink, URI_SKYNET_PREFIX);

    decode_skylink_base64(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_skylink() {
        const SKYLINK: &str = "XABvi7JtJbQSMAcDwnUnmp2FKDPjg8_tTTFP4BwMSxVdEg";

        let bytes = decode_skylink(SKYLINK);

        assert_eq!(
            bytes,
            vec![
                92, 0, 111, 139, 178, 109, 37, 180, 18, 48, 7, 3, 194, 117, 39, 154, 157, 133, 40,
                51, 227, 131, 207, 237, 77, 49, 79, 224, 28, 12, 75, 21, 93, 18
            ]
        );
    }

    #[test]
    fn should_return_correct_specifier() {
        const SPECIFIER: &str = "testing";
        const EXPECTED: &[u8] = &[116, 101, 115, 116, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        assert_eq!(new_specifier(SPECIFIER), EXPECTED);
    }

    #[test]
    fn should_create_v2_skylinks_correctly() {
        // Hard-code expected data from skyd.
        const PUBLIC_KEY: &str = "a1790331b8b41a94644d01a7b482564e7049047812364bcabc32d399ad23f7e2";
        const DATA_KEY: &str = "d321b3c31337047493c9b5a99675e9bdaea44218a31aad2fd7738209e7a5aca1";
        const EXPECTED_SKYLINK: &str = "AQB7zHVDtD-PikoAD_0zzFbWWPcY-IJoJRHXFJcwoU-WvQ";

        let sia_public_key = new_ed25519_public_key(PUBLIC_KEY);
        let skylink = new_skylink_v2(sia_public_key, &decode_hex_to_bytes(DATA_KEY));

        assert_eq!(skylink.to_string(), str_to_bytes(EXPECTED_SKYLINK));
    }
}
