//! Registry functions.

use crate::crypto::{hash_data_key, hash_registry_entry, Signature};
use crate::encoding::{
    decode_hex_bytes_to_bytes, decode_hex_to_bytes, encode_bytes_to_hex_bytes, vec_to_signature,
};
use crate::util::{
    concat_strs, de_string_to_bytes, execute_get, make_url, ser_bytes_to_string, str_to_bytes,
    RequestError, DEFAULT_PORTAL_URL,
};

use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};
use sp_io::offchain;
use sp_runtime::offchain::{self as rt_offchain, http};
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
    HttpError(rt_offchain::HttpError),
    HttpError2(http::Error),
    JsonError(serde_json::Error),
    RequestError(RequestError),
    SignatureError(ed25519_dalek::SignatureError),
    TimeoutError,
    UnexpectedStatus(u16),
    Utf8Error(str::Utf8Error),
}

impl From<http::Error> for SetEntryError {
    fn from(err: http::Error) -> Self {
        Self::HttpError2(err)
    }
}

impl From<rt_offchain::HttpError> for SetEntryError {
    fn from(err: rt_offchain::HttpError) -> Self {
        Self::HttpError(err)
    }
}

impl From<ed25519_dalek::SignatureError> for SetEntryError {
    fn from(err: ed25519_dalek::SignatureError) -> Self {
        Self::SignatureError(err)
    }
}

impl From<RequestError> for SetEntryError {
    fn from(err: RequestError) -> Self {
        Self::RequestError(err)
    }
}

impl From<serde_json::Error> for SetEntryError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
}

impl From<str::Utf8Error> for SetEntryError {
    fn from(err: str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
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

#[derive(Serialize, Default)]
struct SetEntryRequest {
    publickey: PublicKeyRequest,
    #[serde(serialize_with = "ser_bytes_to_string")]
    datakey: Vec<u8>, // Hex string bytes
    revision: u64,
    data: Vec<u8>,      // Raw bytes
    signature: Vec<u8>, // Raw bytes. Serialize and Default not implemented for [u8; 64]
}

#[derive(Serialize, Default)]
struct PublicKeyRequest {
    #[serde(serialize_with = "ser_bytes_to_string")]
    algorithm: Vec<u8>, // String bytes
    key: [u8; 32],
}

pub fn get_entry(
    public_key: &str,
    data_key: &str,
    opts: Option<&GetEntryOptions>,
) -> Result<SignedRegistryEntry, GetEntryError> {
    let default = Default::default();
    let opts = opts.unwrap_or(&default);

    let url = get_entry_url(public_key, data_key, Some(opts))?;

    let resp = execute_get(str::from_utf8(&url)?)?;

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

pub fn set_entry(
    private_key: &str,
    entry: &RegistryEntry,
    opts: Option<&SetEntryOptions>,
) -> Result<(), SetEntryError> {
    let default = Default::default();
    let opts = opts.unwrap_or(&default);

    let private_key_bytes = decode_hex_to_bytes(private_key);
    // TODO: Are the public and private key bytes in the right order?
    // The "private key" is actually a keypair that contains the public and private keys.
    let ed25519_keypair = ed25519_dalek::Keypair::from_bytes(&private_key_bytes)?;

    // Sign the entry.
    let entry_hash = hash_registry_entry(entry)?;
    let signature = ed25519_keypair.sign(&entry_hash);

    let ed25519_public_key = ed25519_keypair.public;
    let data_key_hashed_hex =
        encode_bytes_to_hex_bytes(&hash_data_key(&str::from_utf8(&entry.data_key)?));

    let data = SetEntryRequest {
        publickey: PublicKeyRequest {
            algorithm: str_to_bytes("ed25519"),
            key: ed25519_public_key.to_bytes(),
        },
        datakey: data_key_hashed_hex,
        revision: entry.revision,
        data: entry.data.clone(),
        signature: signature.to_bytes().to_vec(),
    };

    // Serialize the request.
    let body: Vec<u8> = serde_json::to_vec(&data)?;

    // Execute request.
    // Initiate an external HTTP POST request. This is using high-level wrappers from `sp_runtime`.
    let url = make_url(&[opts.portal_url, opts.endpoint_set_entry]);
    let request = rt_offchain::http::Request::post(str::from_utf8(&url)?, vec![body.as_slice()]);

    // Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
    let timeout = offchain::timestamp().add(rt_offchain::Duration::from_millis(3000));

    let pending = request
        .deadline(timeout) // Setting the timeout time
        .send()?; // Sending the request out by the host

    // By default, the http request is async from the runtime perspective. So we are asking the
    // runtime to wait here. The returning value here is a `Result` of `Result`, so we are
    // unwrapping it twice by two `?`
    //
    // ref: https://substrate.dev/rustdocs/v2.0.0-rc3/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
    let response = pending
        .try_wait(timeout)
        .map_err(|_| SetEntryError::TimeoutError)??;

    if response.code >= 400 {
        return Err(SetEntryError::UnexpectedStatus(response.code));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{decode_hex_to_bytes, vec_to_signature};
    use crate::util::str_to_bytes;

    use sp_core::offchain::{testing, OffchainExt};
    use sp_io::TestExternalities;
    use sp_std::str;

    // Hard-code public key and expected encoded values to catch any breaking changes to the
    // encoding code. These values match skynet-js skydb.test.ts.
    const PUBLIC_KEY: &str = "658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
    const PRIVATE_KEY: &str = "7caffac49ac914a541b28723f11776d36ce81e7b9b0c96ccacd1302db429c79c658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
    const DATA_KEY: &str = "app";
    const ENCODED_PK: &str =
        "ed25519%3A658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
    const ENCODED_DK: &str = "7c96a0537ab2aaac9cfe0eca217732f4e10791625b4ab4c17e4d91c8078713b9";

    // Hex-encoded skylink.
    const GET_ENTRY_DATA: &str = "43414241425f31447430464a73787173755f4a34546f644e4362434776744666315579735f3345677a4f6c546367";
    const GET_ENTRY_REVISION: u64 = 11;
    const SIGNATURE: &str = "33d14d2889cb292142614da0e0ff13a205c4867961276001471d13b779fc9032568ddd292d9e0dff69d7b1f28be07972cc9d86da3cecf3adecb6f9b7311af809";

    const EXPECTED_URL: &str = "https://siasky.net/skynet/registry?publickey=ed25519%3A658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9&datakey=7c96a0537ab2aaac9cfe0eca217732f4e10791625b4ab4c17e4d91c8078713b9&timeout=5";
    const ENTRY_DATA_RESPONSE_JSON: &str = "{ \"data\": \"43414241425f31447430464a73787173755f4a34546f644e4362434776744666315579735f3345677a4f6c546367\", \"revision\": 11, \"signature\": \"33d14d2889cb292142614da0e0ff13a205c4867961276001471d13b779fc9032568ddd292d9e0dff69d7b1f28be07972cc9d86da3cecf3adecb6f9b7311af809\" }";

    const SET_ENTRY_DATA: &[u8] = &[
        8, 0, 64, 7, 253, 67, 183, 65, 73, 179, 26, 172, 187, 242, 120, 78, 135, 77, 9, 176, 134,
        190, 209, 95, 213, 76, 172, 255, 113, 32, 204, 233, 83, 114,
    ];
    const SET_ENTRY_REVISION: u64 = 0;
    const SET_ENTRY_REQUEST_JSON: &str = "{\"publickey\":{\"algorithm\":\"ed25519\",\"key\":[101,139,144,13,245,94,152,60,232,95,63,159,178,160,136,213,104,171,81,78,123,189,165,28,251,251,22,234,148,83,120,217]},\"datakey\":\"7c96a0537ab2aaac9cfe0eca217732f4e10791625b4ab4c17e4d91c8078713b9\",\"revision\":0,\"data\":[8,0,64,7,253,67,183,65,73,179,26,172,187,242,120,78,135,77,9,176,134,190,209,95,213,76,172,255,113,32,204,233,83,114],\"signature\":[53,132,90,36,67,157,35,167,252,203,42,224,40,223,82,144,217,94,138,166,102,186,94,145,231,125,90,63,149,153,83,13,239,95,65,219,84,143,63,193,195,112,106,10,247,33,232,122,169,85,156,149,109,180,204,75,249,179,251,183,160,230,235,8]}";

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
            assert_eq!(returned_signed_entry, get_entry_make_signed_entry());
        })
    }

    #[test]
    fn should_sign_and_set_entry() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "POST".into(),
            uri: "https://siasky.net/skynet/registry".into(),
            body: SET_ENTRY_REQUEST_JSON.into(),
            response: Some("".into()),
            sent: true,
            ..Default::default()
        });

        t.execute_with(|| {
            let entry = set_entry_make_entry();
            // Set entry.
            let _ = set_entry(PRIVATE_KEY, &entry, None).unwrap();
        })
    }

    fn get_entry_make_entry() -> RegistryEntry {
        RegistryEntry {
            data_key: str_to_bytes(DATA_KEY),
            data: decode_hex_to_bytes(GET_ENTRY_DATA),
            revision: GET_ENTRY_REVISION,
        }
    }

    fn get_entry_make_signed_entry() -> SignedRegistryEntry {
        SignedRegistryEntry {
            entry: Some(get_entry_make_entry()),
            signature: Some(vec_to_signature(decode_hex_to_bytes(SIGNATURE))),
        }
    }

    fn set_entry_make_entry() -> RegistryEntry {
        RegistryEntry {
            data_key: str_to_bytes(DATA_KEY),
            data: SET_ENTRY_DATA.to_vec(),
            revision: SET_ENTRY_REVISION,
        }
    }
}
