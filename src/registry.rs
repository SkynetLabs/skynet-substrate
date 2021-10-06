//! Registry functions.

use crate::crypto::{Signature, hash_data_key};
use crate::encoding::encode_bytes_to_hex_bytes;
use crate::util::{concat_strs, execute_request, make_url, RequestError, DEFAULT_PORTAL_URL};

use sp_std::{prelude::Vec, str};

/// The get entry timeout. Not configurable. Not exported as this is planned to be removed.
const DEFAULT_GET_ENTRY_TIMEOUT: &str = "5";

const ED25519_PREFIX_URL_ENCODED: &str = "ed25519%3A";

#[derive(Debug)]
pub enum GetEntryError {
    RequestError(RequestError),
    Utf8Error(str::Utf8Error),
}

impl From<str::Utf8Error> for GetEntryError {
    fn from(err: str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

#[derive(Debug)]
pub enum SetEntryError {
    RequestError(RequestError),
    Utf8Error(str::Utf8Error),
}

#[derive(Debug)]
pub struct GetEntryOptions<'a> {
    pub portal_url: &'a str,
    pub endpoint_get_entry: &'a str,
}

impl Default for GetEntryOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_get_entry: "/skynet/registry",
        }
    }
}

#[derive(Debug)]
pub struct SetEntryOptions<'a> {
    pub portal_url: &'a str,
    pub endpoint_set_entry: &'a str,
}

impl Default for SetEntryOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_set_entry: "/skynet/registry",
        }
    }
}

/// Registry entry.
pub struct RegistryEntry {
    /// The key of the data for the given entry.
    data_key: Vec<u8>,
    /// The data stored in the entry.
    data: Vec<u8>,
    /// The revision number for the entry.
    revision: u64,
}

/// Signed registry entry.
pub struct SignedRegistryEntry {
    /// The signature of the registry entry.
    entry: Option<RegistryEntry>,
    /// The registry entry.
    signature: Option<Signature>,
}

// pub fn get_entry(
//     public_key: &str,
//     data_key: &str,
//     opts: Option<&GetEntryOptions>,
// ) -> Result<SignedRegistryEntry, GetEntryError> {
//     let default = Default::default();
//     let opts = opts.unwrap_or(&default);

//     let url = get_entry_url(public_key, data_key, opts);
// }

pub fn get_entry_url(
    public_key: &str,
    data_key: &str,
    opts: Option<&GetEntryOptions>,
) -> Result<Vec<u8>, GetEntryError> {
    let default = Default::default();
    let opts = opts.unwrap_or(&default);

    let url = make_url(&[opts.portal_url, opts.endpoint_get_entry]);

    let data_key_hash = hash_data_key(data_key);
    let data_key_hash_hex = encode_bytes_to_hex_bytes(&data_key_hash);

    Ok(concat_strs(&[
        str::from_utf8(&url)?,
        "?publickey=",
        ED25519_PREFIX_URL_ENCODED,
        public_key,
        "&datakey=",
        str::from_utf8(&data_key_hash_hex)?,
        "&timeout=",
        DEFAULT_GET_ENTRY_TIMEOUT,
    ]))
}

// pub fn set_entry(
//     private_key: &str,
//     entry: RegistryEntry,
//     opts: Option<&SetEntryOptions>,
// ) -> Result<(), SetEntryError> {
//     let default = Default::default();
//     let opts = opts.unwrap_or(&default);
// }

#[cfg(test)]
mod tests {
    use super::*;
    use sp_std::str;

    // Hard-code public key and expected encoded values to catch any breaking changes to the
    // encoding code.
    const PUBLIC_KEY: &str = "c1197e1275fbf570d21dde01a00af83ed4a743d1884e4a09cebce0dd21ae254c";
    const DATA_KEY: &str = "app";
    const ENCODED_PK: &str =
        "ed25519%3Ac1197e1275fbf570d21dde01a00af83ed4a743d1884e4a09cebce0dd21ae254c";
    const ENCODED_DK: &str = "7c96a0537ab2aaac9cfe0eca217732f4e10791625b4ab4c17e4d91c8078713b9";

    // Should generate the correct registry url for the given entry
    #[test]
    fn should_generate_correct_entry_url() {
        let expected_url: Vec<u8> = concat_strs(&[
            DEFAULT_PORTAL_URL,
            "/skynet/registry",
            "?publickey=",
            ENCODED_PK,
            "&datakey=",
            ENCODED_DK,
            "&timeout=5",
        ]);

        let url = get_entry_url(PUBLIC_KEY, DATA_KEY, None).unwrap();

        assert_eq!(url, expected_url);
    }
}
