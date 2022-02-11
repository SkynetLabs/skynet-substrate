//! Registry functions.

use crate::crypto::{hash_data_key, hash_registry_entry, Signature};
use crate::encoding::{
    decode_hex_bytes_to_bytes, decode_hex_to_bytes, encode_bytes_to_hex_bytes, vec_to_signature,
};
use crate::skylink::{decode_skylink, new_ed25519_public_key, new_skylink_v2};
use crate::util::{
    concat_strs, de_string_to_bytes, execute_get, format_skylink, make_url, ser_bytes_to_string,
    str_to_bytes, RequestError, DEFAULT_PORTAL_URL,
};

use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};
use sp_io::offchain;
use sp_runtime::offchain::{self as rt_offchain, http};
use sp_std::{prelude::Vec, str, vec};

/// The get entry timeout. Not configurable. Not exported as this is planned to be removed.
const DEFAULT_GET_ENTRY_TIMEOUT: &str = "5";

const ED25519_PREFIX_URL_ENCODED: &str = "ed25519%3A";

/// The maximum length for entry data when setting entry data.
const MAX_ENTRY_LENGTH: usize = 70;

// ======
// ERRORS
// ======

/// Get entry error.
#[derive(Debug)]
pub enum GetEntryError {
    /// JSON error.
    JsonError(serde_json::Error),
    /// Request error.
    RequestError(RequestError),
    /// Signature error.
    SignatureError(ed25519_dalek::SignatureError),
    /// UTF8 error.
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

/// Set entry error.
#[derive(Debug)]
pub enum SetEntryError {
    /// HTTP error.
    HttpError(rt_offchain::HttpError),
    /// HTTP error.
    HttpError2(http::Error),
    /// JSON error.
    JsonError(serde_json::Error),
    /// Request error.
    RequestError(RequestError),
    /// Signature error.
    SignatureError(ed25519_dalek::SignatureError),
    /// Timeout error.
    TimeoutError,
    /// Unexpected status.
    UnexpectedStatus(u16),
    /// UTF8 error.
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

/// Set entry data error.
#[derive(Debug)]
pub enum SetEntryDataError {
    /// Data exceeds max allowed width.
    DataTooLongError(usize),
    GetEntryError(GetEntryError),
    SetEntryError(SetEntryError),
    /// Signature error.
    SignatureError(ed25519_dalek::SignatureError),
    /// UTF8 error.
    Utf8Error(str::Utf8Error),
}

impl From<GetEntryError> for SetEntryDataError {
    fn from(err: GetEntryError) -> Self {
        Self::GetEntryError(err)
    }
}

impl From<SetEntryError> for SetEntryDataError {
    fn from(err: SetEntryError) -> Self {
        Self::SetEntryError(err)
    }
}

impl From<ed25519_dalek::SignatureError> for SetEntryDataError {
    fn from(err: ed25519_dalek::SignatureError) -> Self {
        Self::SignatureError(err)
    }
}

impl From<str::Utf8Error> for SetEntryDataError {
    fn from(err: str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

// =======
// OPTIONS
// =======

/// Get entry options.
#[derive(Debug)]
pub struct GetEntryOptions<'a> {
    /// The portal URL.
    pub portal_url: &'a str,
    /// The endpoint to contact.
    pub endpoint_get_entry: &'a str,
    /// Optional custom cookie.
    pub custom_cookie: Option<&'a str>,
}

impl Default for GetEntryOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_get_entry: "/skynet/registry",
            custom_cookie: None,
        }
    }
}

/// Set entry options.
#[derive(Debug)]
pub struct SetEntryOptions<'a> {
    /// The portal URL.
    pub portal_url: &'a str,
    /// The endpoint to contact.
    pub endpoint_set_entry: &'a str,
    /// Optional custom cookie.
    pub custom_cookie: Option<&'a str>,
}

impl Default for SetEntryOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_set_entry: "/skynet/registry",
            custom_cookie: None,
        }
    }
}

/// Set entry data options.
#[derive(Debug, Default)]
pub struct SetEntryDataOptions<'a> {
    pub get_entry_opts: Option<&'a GetEntryOptions<'a>>,
    pub set_entry_opts: Option<&'a SetEntryOptions<'a>>,
}

// =====
// TYPES
// =====

#[derive(Debug, PartialEq)]
pub struct EntryData {
    pub data: Option<Vec<u8>>,
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

// =========
// FUNCTIONS
// =========

/// Gets registry entry for `public_key` and `data_key`.
pub fn get_entry(
    public_key: &str,
    data_key: &str,
    opts: Option<&GetEntryOptions>,
) -> Result<SignedRegistryEntry, GetEntryError> {
    let default = Default::default();
    let opts = opts.unwrap_or(&default);

    let url = get_entry_url(public_key, data_key, Some(opts))?;

    let resp = match execute_get(str::from_utf8(&url)?, opts.custom_cookie) {
        // If a 404 status was found, return a null entry.
        Err(RequestError::UnexpectedStatus(404)) => {
            return Ok(SignedRegistryEntry {
                entry: None,
                signature: None,
            })
        }
        x => x,
    }?;

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
    ed25519_public_key.verify_strict(&message, &ed25519_dalek::Signature::new(signature))?;

    Ok(signed_entry)
}

/// Gets registry entry URL for `public_key` and `data_key`.
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

/// Sets registry `entry` at `private_key`.
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
        encode_bytes_to_hex_bytes(&hash_data_key(str::from_utf8(&entry.data_key)?));

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
    let mut request =
        rt_offchain::http::Request::post(str::from_utf8(&url)?, vec![body.as_slice()]);

    if let Some(cookie) = opts.custom_cookie {
        request = request.add_header("Cookie", cookie);
    }

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

/// Sets the datalink for the entry at the given private key and data key.
pub fn set_data_link(
    private_key: &str,
    data_key: &str,
    data_link: &str,
    opts: Option<&SetEntryDataOptions>,
) -> Result<(), SetEntryDataError> {
    let data = decode_skylink(data_link);

    set_entry_data(private_key, data_key, &data, opts)?;
    Ok(())
}

/// Sets the raw entry data at the given private key and data key.
pub fn set_entry_data(
    private_key: &str,
    data_key: &str,
    data: &[u8],
    opts: Option<&SetEntryDataOptions>,
) -> Result<EntryData, SetEntryDataError> {
    if data.len() > MAX_ENTRY_LENGTH {
        return Err(SetEntryDataError::DataTooLongError(data.len()));
    }

    let default = Default::default();
    let opts = opts.unwrap_or(&default);

    // Get the public key.
    let private_key_bytes = decode_hex_to_bytes(private_key);
    // TODO: Are the public and private key bytes in the right order?
    // The "private key" is actually a keypair that contains the public and private keys.
    let ed25519_keypair = ed25519_dalek::Keypair::from_bytes(&private_key_bytes)?;
    let public_key_bytes = ed25519_keypair.public.to_bytes();
    let public_key_hex_bytes = encode_bytes_to_hex_bytes(&public_key_bytes);
    let public_key = str::from_utf8(&public_key_hex_bytes)?;

    // Get the entry in order to get the revision number.
    let signed_entry = get_entry(public_key, data_key, opts.get_entry_opts)?;
    // TODO: check for overflow
    let revision = if let Some(entry) = signed_entry.entry {
        entry.revision + 1
    } else {
        0
    };

    // Construct the entry.
    let entry = RegistryEntry {
        data_key: str_to_bytes(data_key),
        data: data.to_vec(),
        revision,
    };

    // Set the entry.
    set_entry(private_key, &entry, opts.set_entry_opts)?;

    Ok(EntryData {
        data: Some(data.to_vec()),
    })
}

/// Gets the entry link for the entry at the given `public_key` and `data_key`. This link stays the
/// same even if the content at the entry changes.
pub fn get_entry_link(
    public_key: &str,
    data_key: &str,
    _opts: Option<&GetEntryOptions>,
) -> Result<Vec<u8>, GetEntryError> {
    // let default = Default::default();
    // let opts = opts.unwrap_or(&default);

    let sia_public_key = new_ed25519_public_key(public_key);
    let tweak = hash_data_key(data_key);

    let skylink = new_skylink_v2(sia_public_key, &tweak).to_string();
    Ok(format_skylink(&skylink))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{decode_hex_to_bytes, vec_to_signature};
    use crate::util::str_to_bytes;

    use sp_core::offchain::{testing, OffchainWorkerExt};
    use sp_io::TestExternalities;
    use sp_std::str;

    // Hard-code public key and expected encoded values to catch any breaking changes to the
    // encoding code. These values match skynet-js skydb.test.ts.
    const PUBLIC_KEY: &str = "658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
    const PRIVATE_KEY: &str = "7caffac49ac914a541b28723f11776d36ce81e7b9b0c96ccacd1302db429c79c658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
    const DATA_KEY: &str = "app";

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
        t.register_extension(OffchainWorkerExt::new(offchain));

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
            let entry = RegistryEntry {
                data_key: str_to_bytes(DATA_KEY),
                data: decode_hex_to_bytes(GET_ENTRY_DATA),
                revision: GET_ENTRY_REVISION,
            };
            let signed_entry = SignedRegistryEntry {
                entry: Some(entry),
                signature: Some(vec_to_signature(decode_hex_to_bytes(SIGNATURE))),
            };
            assert_eq!(returned_signed_entry, signed_entry);
        })
    }

    // TODO: How to simulate a 404 response?
    // #[test]
    // fn should_return_none_if_entry_not_found() {
    //     let (offchain, state) = testing::TestOffchainExt::new();
    //     let mut t = TestExternalities::default();
    //     t.register_extension(OffchainWorkerExt::new(offchain));

    //     // Add expected request.
    //     state.write().expect_request(testing::PendingRequest {
    //         method: "GET".into(),
    //         uri: EXPECTED_URL.into(),
    //         response: None,
    //         sent: true,
    //         ..Default::default()
    //     });

    //     t.execute_with(|| {
    //         // Get entry.
    //         let returned_signed_entry = get_entry(PUBLIC_KEY, DATA_KEY, None).unwrap();

    //         // Check the response.
    // let null_entry = SignedRegistryEntry {
    //     entry: None,
    //     signature: None,
    // };
    //         assert_eq!(returned_signed_entry, null_entry);
    //     });
    // }

    #[test]
    fn should_sign_and_set_entry() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

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
            let entry = RegistryEntry {
                data_key: str_to_bytes(DATA_KEY),
                data: SET_ENTRY_DATA.to_vec(),
                revision: SET_ENTRY_REVISION,
            };
            // Set entry.
            let _ = set_entry(PRIVATE_KEY, &entry, None).unwrap();
        })
    }

    #[test]
    fn should_set_data_link() {
        const DATA_LINK: &str = "sia://AAA6Z7R0sjreLCr35fJKhMXuc8CE6mxRhkHQtmgtJGzqvw";
        const SET_ENTRY_REQUEST_JSON: &str = "{\"publickey\":{\"algorithm\":\"ed25519\",\"key\":[101,139,144,13,245,94,152,60,232,95,63,159,178,160,136,213,104,171,81,78,123,189,165,28,251,251,22,234,148,83,120,217]},\"datakey\":\"7c96a0537ab2aaac9cfe0eca217732f4e10791625b4ab4c17e4d91c8078713b9\",\"revision\":12,\"data\":[0,0,58,103,180,116,178,58,222,44,42,247,229,242,74,132,197,238,115,192,132,234,108,81,134,65,208,182,104,45,36,108,234,191],\"signature\":[230,73,17,53,225,37,252,223,75,109,202,5,44,45,201,52,121,240,88,90,19,152,205,231,144,102,84,116,33,37,14,161,175,164,154,149,217,169,202,41,231,14,246,177,148,13,87,79,63,4,68,103,39,101,246,148,163,249,164,91,163,243,12,9]}";

        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: EXPECTED_URL.into(),
            response: Some(ENTRY_DATA_RESPONSE_JSON.into()),
            sent: true,
            ..Default::default()
        });

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
            // Set data link.
            let _ = set_data_link(PRIVATE_KEY, DATA_KEY, DATA_LINK, None).unwrap();
        })
    }

    #[test]
    fn should_update_entry_data() {
        const DATA: &[u8] = &[1, 2, 3];
        const SET_ENTRY_REQUEST_JSON: &str = "{\"publickey\":{\"algorithm\":\"ed25519\",\"key\":[101,139,144,13,245,94,152,60,232,95,63,159,178,160,136,213,104,171,81,78,123,189,165,28,251,251,22,234,148,83,120,217]},\"datakey\":\"7c96a0537ab2aaac9cfe0eca217732f4e10791625b4ab4c17e4d91c8078713b9\",\"revision\":12,\"data\":[1,2,3],\"signature\":[89,214,206,198,28,243,240,118,171,61,137,4,89,6,26,79,112,54,72,239,109,148,187,171,72,112,21,158,57,121,62,183,17,97,231,54,169,132,50,222,130,255,131,162,121,139,27,55,65,98,114,241,150,197,182,48,76,230,221,58,165,210,195,4]}";

        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: EXPECTED_URL.into(),
            response: Some(ENTRY_DATA_RESPONSE_JSON.into()),
            sent: true,
            ..Default::default()
        });

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
            // Set entry data.
            let returned_data = set_entry_data(PRIVATE_KEY, DATA_KEY, DATA, None).unwrap();

            // Check the response.
            assert_eq!(
                returned_data,
                EntryData {
                    data: Some(DATA.to_vec())
                }
            );
        })
    }

    #[test]
    fn should_get_the_correct_entry_link_1() {
        const PUBLIC_KEY: &str = "a1790331b8b41a94644d01a7b482564e7049047812364bcabc32d399ad23f7e2";
        const DATA_KEY: &str = "d321b3c31337047493c9b5a99675e9bdaea44218a31aad2fd7738209e7a5aca1";
        const EXPECTED_ENTRY_LINK: &str = "sia://AQBT237lo425ivk3Si6sOKretXxsDwO6DT1M0_Ui3oT0OA";

        let entry_link = get_entry_link(PUBLIC_KEY, DATA_KEY, None).unwrap();

        assert_eq!(str::from_utf8(&entry_link).unwrap(), EXPECTED_ENTRY_LINK);
    }

    #[test]
    fn should_get_the_correct_entry_link_2() {
        const PUBLIC_KEY: &str = "658b900df55e983ce85f3f9fb2a088d568ab514e7bbda51cfbfb16ea945378d9";
        const DATA_KEY: &str = "historical-block-weights";
        const EXPECTED_ENTRY_LINK: &str = "sia://AQBLxu38T6ceg0ey_UUbexZzo_Y8AwFvIdYePG96FSVU1A";

        let entry_link = get_entry_link(PUBLIC_KEY, DATA_KEY, None).unwrap();

        assert_eq!(str::from_utf8(&entry_link).unwrap(), EXPECTED_ENTRY_LINK);
    }
}
