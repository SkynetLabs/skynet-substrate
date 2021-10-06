//! Registry functions.

use crate::crypto::{hash_data_key, hash_registry_entry, Signature};
use crate::encoding::{
    decode_hex_bytes_to_bytes, decode_hex_to_bytes, encode_bytes_to_hex_bytes, vec_to_signature,
};
use crate::util::{
    concat_strs, de_string_to_bytes, execute_request, make_url, RequestError, DEFAULT_PORTAL_URL, str_to_bytes
};

use serde::{Deserialize, Deserializer};
use sp_std::{prelude::Vec, str};

/// The get entry timeout. Not configurable. Not exported as this is planned to be removed.
const DEFAULT_GET_ENTRY_TIMEOUT: &str = "5";

const ED25519_PREFIX_URL_ENCODED: &str = "ed25519%3A";

#[derive(Debug)]
pub enum GetEntryError {
    JsonError(serde_json::Error),
    RequestError(RequestError),
    SignatureError(ed25519_dalek::SignatureError),
    Utf8Error(str::Utf8Error),
}

impl From<ed25519_dalek::SignatureError> for GetEntryError {
    fn from(err: ed25519_dalek::SignatureError) -> Self {
        Self::SignatureError(err)
    }
}

impl From<RequestError> for GetEntryError {
    fn from(err: RequestError) -> Self {
        Self::RequestError(err)
    }
}

impl From<serde_json::Error> for GetEntryError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
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
#[derive(Debug, PartialEq)]
pub struct RegistryEntry {
    /// The key of the data for the given entry.
    pub data_key: Vec<u8>, // UTF8 string, stored as bytes
    /// The data stored in the entry.
    pub data: Vec<u8>, // Raw bytes
    /// The revision number for the entry.
    pub revision: u64,
}

/// Signed registry entry.
#[derive(Debug, PartialEq)]
pub struct SignedRegistryEntry {
    /// The signature of the registry entry.
    pub entry: Option<RegistryEntry>,
    /// The registry entry.
    pub signature: Option<Signature>,
}

// ref: https://serde.rs/container-attrs.html#crate
#[derive(Deserialize, Default)]
struct GetEntryResponse {
    // Specify our own deserializing function to convert string to vector of bytes.
    #[serde(deserialize_with = "de_string_to_bytes")]
    data: Vec<u8>,
    revision: u64,
    #[serde(deserialize_with = "de_string_to_bytes")]
    signature: Vec<u8>,
}

pub fn get_entry(
    public_key: &str,
    data_key: &str,
    opts: Option<&GetEntryOptions>,
) -> Result<SignedRegistryEntry, GetEntryError> {
    let default = Default::default();
    let opts = opts.unwrap_or(&default);

    let url = get_entry_url(public_key, data_key, Some(opts))?;

    let resp = execute_request(str::from_utf8(&url)?)?;

    // Read the response body and collect it to a vector of bytes.
    let resp_bytes = resp.body().collect::<Vec<u8>>();
    // Convert the bytes to a str.
    let resp_str = str::from_utf8(&resp_bytes)?;
    // Parse the str as JSON and store it in GetEntryResponse.
    let get_entry_response: GetEntryResponse = serde_json::from_str(resp_str)?;

    let data = decode_hex_bytes_to_bytes(&get_entry_response.data);
    let signature = vec_to_signature(decode_hex_bytes_to_bytes(&get_entry_response.signature));

    let entry = RegistryEntry {
        data_key: str_to_bytes(data_key),
        data,
        revision: get_entry_response.revision,
    };
    let message = hash_registry_entry(&entry)?;
    let signed_entry = SignedRegistryEntry {
        entry: Some(entry),
        signature: Some(signature),
    };

    // TODO
    let public_key_bytes = decode_hex_to_bytes(public_key);
    let ed25519_public_key = ed25519_dalek::PublicKey::from_bytes(&public_key_bytes)?;

    // Verify the signature, return an error if it could not be verified.
    Ok(ed25519_public_key
        .verify_strict(&message, &ed25519_dalek::Signature::new(signature))
        .map(|()| signed_entry)?)
}

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
    use crate::encoding::{decode_hex_to_bytes, vec_to_signature};
    use crate::util::str_to_bytes;

    use sp_core::offchain::{testing, OffchainExt};
    use sp_io::TestExternalities;
    use sp_std::str;

    // Hard-code public key and expected encoded values to catch any breaking changes to the
    // encoding code.
    const PUBLIC_KEY: &str = "658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
    const DATA_KEY: &str = "app";
    const ENCODED_PK: &str =
        "ed25519%3A658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
    const ENCODED_DK: &str = "7c96a0537ab2aaac9cfe0eca217732f4e10791625b4ab4c17e4d91c8078713b9";

    // Hex-encoded skylink.
    const DATA: &str = "43414241425f31447430464a73787173755f4a34546f644e4362434776744666315579735f3345677a4f6c546367";
    const REVISION: u64 = 11;
    const SIGNATURE: &str = "33d14d2889cb292142614da0e0ff13a205c4867961276001471d13b779fc9032568ddd292d9e0dff69d7b1f28be07972cc9d86da3cecf3adecb6f9b7311af809";

    const EXPECTED_URL: &str = "https://siasky.net/skynet/registry?publickey=ed25519%3A658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9&datakey=7c96a0537ab2aaac9cfe0eca217732f4e10791625b4ab4c17e4d91c8078713b9&timeout=5";
    const ENTRY_DATA_RESPONSE_JSON: &str = "{ \"data\": \"43414241425f31447430464a73787173755f4a34546f644e4362434776744666315579735f3345677a4f6c546367\", \"revision\": 11, \"signature\": \"33d14d2889cb292142614da0e0ff13a205c4867961276001471d13b779fc9032568ddd292d9e0dff69d7b1f28be07972cc9d86da3cecf3adecb6f9b7311af809\" }";

    // Should generate the correct registry url for the given entry
    #[test]
    fn should_generate_correct_entry_url() {
        let url = get_entry_url(PUBLIC_KEY, DATA_KEY, None).unwrap();

        assert_eq!(url, str_to_bytes(EXPECTED_URL));
    }

    #[test]
    fn should_get_and_verify_entry() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainExt::new(offchain));

        let url = get_entry_url(PUBLIC_KEY, DATA_KEY, None).unwrap();

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: EXPECTED_URL.into(),
            response: Some(ENTRY_DATA_RESPONSE_JSON.into()),
            sent: true,
            ..Default::default()
        });

        t.execute_with(|| {
            // Get entry.
            let returned_signed_entry = get_entry(PUBLIC_KEY, DATA_KEY, None).unwrap();

            // Check the response.
            assert_eq!(returned_signed_entry, make_signed_entry());
        })
    }

    fn make_entry() -> RegistryEntry {
        RegistryEntry {
            data_key: str_to_bytes(DATA_KEY),
            data: decode_hex_to_bytes(DATA),
            revision: REVISION,
        }
    }

    fn make_signed_entry() -> SignedRegistryEntry {
        SignedRegistryEntry {
            entry: Some(make_entry()),
            signature: Some(vec_to_signature(decode_hex_to_bytes(SIGNATURE))),
        }
    }
}
