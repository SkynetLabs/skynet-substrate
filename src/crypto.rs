use crate::encoding;
use crate::registry::RegistryEntry;
use crate::util::concat_bytes;

use sp_core::hashing;
use sp_std::{prelude::Vec, str};

pub type Signature = [u8; SIGNATURE_LENGTH];

pub const HASH_LENGTH: usize = 32;

pub const PUBLIC_KEY_LENGTH: usize = ed25519_dalek::PUBLIC_KEY_LENGTH * 2;

pub const PRIVATE_KEY_LENGTH: usize = ed25519_dalek::SECRET_KEY_LENGTH * 2;

pub const SIGNATURE_LENGTH: usize = ed25519_dalek::SIGNATURE_LENGTH;

fn hash_all(args: &[&[u8]]) -> Vec<u8> {
    let bytes = concat_bytes(args);
    hashing::blake2_256(&bytes).to_vec()
}

pub fn hash_data_key(data_key: &str) -> Vec<u8> {
    let bytes = encoding::encode_str(data_key);
    hashing::blake2_256(&bytes).to_vec()
}

pub fn hash_registry_entry(registry_entry: &RegistryEntry) -> Result<Vec<u8>, str::Utf8Error> {
    let data_key_bytes = hash_data_key(str::from_utf8(&registry_entry.data_key)?);

    let data_bytes = encoding::encode_prefixed_bytes(&registry_entry.data);

    Ok(hash_all(&[
        &data_key_bytes,
        &data_bytes,
        &encoding::encode_number(registry_entry.revision),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::encode_bytes_to_hex_bytes;
    use crate::util::str_to_bytes;

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

    // Hard-code expected values to catch any breaking changes.
    #[test]
    fn should_match_siad_for_equal_input() {
        // The hash generated by siad with the same input parameters
        const H: &str = "788dddf5232807611557a3dc0fa5f34012c2650526ba91d55411a2b04ba56164";

        let hash = hash_registry_entry(&RegistryEntry {
            data_key: str_to_bytes("HelloWorld"),
            data: str_to_bytes("abc"),
            revision: 123456789,
        })
        .unwrap();

        assert_eq!(encode_bytes_to_hex_bytes(&hash), str_to_bytes(H));
    }

    // Hard-code expected values to catch any breaking changes.
    #[test]
    fn should_match_siad_for_equal_input_when_data_key_and_data_include_unicode() {
        // The hash generated by siad with the same input parameters
        const H: &str = "ff3b430675a0666e7461bc34aec9f66e21183d061f0b8232dd28ca90cc6ea5ca";

        let hash = hash_registry_entry(&RegistryEntry {
            data_key: str_to_bytes("HelloWorld π"),
            data: str_to_bytes("abc π"),
            revision: 123456789,
        })
        .unwrap();

        assert_eq!(encode_bytes_to_hex_bytes(&hash), str_to_bytes(H));
    }
}