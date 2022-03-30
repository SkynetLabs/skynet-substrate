//! Upload functions.

use crate::request::CommonOptions;
use crate::util::{
    concat_bytes, concat_strs, de_string_to_bytes, format_skylink, make_url, str_to_bytes,
};

use serde::Deserialize;
use sp_io::offchain;
use sp_runtime::offchain::{self as rt_offchain, http};
use sp_std::{prelude::Vec, str, vec};

const PORTAL_FILE_FIELD_NAME: &str = "file";

/// Upload error.
#[derive(Debug)]
pub enum UploadError {
    /// HTTP error.
    HttpError(rt_offchain::HttpError),
    /// HTTP error.
    HttpError2(http::Error),
    /// JSON error.
    JsonError(serde_json::Error),
    /// Timeout error.
    TimeoutError,
    /// Unexpected status.
    UnexpectedStatus(u16),
    /// UTF8 error.
    Utf8Error(str::Utf8Error),
}

impl From<http::Error> for UploadError {
    fn from(err: http::Error) -> Self {
        Self::HttpError2(err)
    }
}

impl From<rt_offchain::HttpError> for UploadError {
    fn from(err: rt_offchain::HttpError) -> Self {
        Self::HttpError(err)
    }
}

impl From<serde_json::Error> for UploadError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
}

impl From<str::Utf8Error> for UploadError {
    fn from(err: str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

/// Upload options.
#[derive(Debug)]
pub struct UploadOptions<'a> {
    /// Common options.
    pub common: CommonOptions<'a>,
    /// The endpoint to contact.
    pub endpoint_upload: &'a str,
    /// Timeout.
    pub timeout: u64,
}

impl Default for UploadOptions<'_> {
    fn default() -> Self {
        Self {
            common: Default::default(),
            endpoint_upload: "/skynet/skyfile",
            timeout: 3_000,
        }
    }
}

// ref: https://serde.rs/container-attrs.html#crate
#[derive(Deserialize, Default)]
struct UploadResponse {
    // Specify our own deserializing function to convert JSON string to vector of bytes
    #[serde(deserialize_with = "de_string_to_bytes")]
    skylink: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    #[allow(dead_code)]
    merkleroot: Vec<u8>,
    #[allow(dead_code)]
    bitfield: u16,
}

/// Upload `bytes` to a file with `filename`.
pub fn upload_bytes(
    bytes: &[u8],
    filename: &str,
    opts: Option<&UploadOptions>,
) -> Result<Vec<u8>, UploadError> {
    let default = &Default::default();
    let opts = opts.unwrap_or(default);

    // Construct the URL.
    let url = make_url(&[opts.common.portal_url, opts.endpoint_upload]);

    // Build the request body boundary.

    let timestamp: u64 = offchain::timestamp().unix_millis();

    // Make a 68-character boundary.
    // TODO: Use a random boundary? Wasn't sure how to do that in Substrate.
    let mut strs = Vec::<&str>::with_capacity(65);
    for i in 0..64 {
        strs.push(if timestamp & (1 << (63 - i)) > 0 {
            "1"
        } else {
            "0"
        })
    }
    strs.push("----");
    let boundary = concat_strs(&strs);

    // Build the request body.

    let mime = "application/octet-stream";

    let disposition = concat_strs(&[
        "form-data; name=\"",
        PORTAL_FILE_FIELD_NAME,
        "\"; filename=\"",
        filename,
        "\"",
    ]);
    let headers = concat_strs(&[
        "Content-Disposition: ",
        str::from_utf8(&disposition)?,
        "\r\nContent-Type: ",
        mime,
        "\r\n",
    ]);

    let body_bytes = concat_bytes(&[
        &str_to_bytes("--"),
        &boundary,
        &str_to_bytes("\r\n"),
        &headers,
        &str_to_bytes("\r\n"),
        bytes,
        &str_to_bytes("\r\n"),
        &str_to_bytes("--"),
        &boundary,
        &str_to_bytes("--\r\n"),
    ]);

    let content_type = concat_strs(&[
        "multipart/form-data; boundary=\"",
        str::from_utf8(&boundary)?,
        "\"",
    ]);

    // Initiate an external HTTP POST request. This is using high-level wrappers from `sp_runtime`.
    let mut request = rt_offchain::http::Request::post(str::from_utf8(&url)?, vec![body_bytes])
        .add_header("Content-Type", str::from_utf8(&content_type)?);

    // NOTE: Can't use `add_headers` here because of a mismatched type error.
    if let Some(cookie) = opts.common.custom_cookie {
        request = request.add_header("Cookie", cookie);
    }
    if let Some(key) = opts.common.skynet_api_key {
        request = request.add_header("Skynet-Api-Key", key);
    }

    // Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
    let timeout = offchain::timestamp().add(rt_offchain::Duration::from_millis(opts.timeout));

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
        .map_err(|_| UploadError::TimeoutError)??;

    if response.code >= 400 {
        return Err(UploadError::UnexpectedStatus(response.code));
    }

    // Read the response body and collect it to a vector of bytes.
    let resp_bytes = response.body().collect::<Vec<u8>>();
    // Convert the bytes to a str.
    let resp_str = str::from_utf8(&resp_bytes)?;
    // Parse the str as JSON and store it in UploadResponse.
    let upload_response: UploadResponse = serde_json::from_str(resp_str)?;
    Ok(format_skylink(&upload_response.skylink))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::str_to_bytes;

    use sp_core::offchain::{testing, OffchainWorkerExt};
    use sp_io::TestExternalities;

    const EXPECTED_DATA_LINK: &str = "sia://MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ";
    const DATA: &str = "foo";
    const FILE_NAME: &str = "barfile";
    const REQUEST_BODY: &str = "--0000000000000000000000000000000000000000000000000000000000000000----\r\nContent-Disposition: form-data; name=\"file\"; filename=\"barfile\"\r\nContent-Type: application/octet-stream\r\n\r\nfoo\r\n--0000000000000000000000000000000000000000000000000000000000000000------\r\n";
    const RESPONSE_JSON: &str = "{\"skylink\": \"MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ\", \"merkleroot\": \"foo\", \"bitfield\": 1028}";
    const CONTENT_TYPE_MULTIPART: &str = "multipart/form-data; boundary=\"0000000000000000000000000000000000000000000000000000000000000000----\"";

    const JWT_COOKIE: &str = "MTYz...=="; // Don't use a full JWT as it's quite long.
    const SKYNET_API_KEY: &str = "foo";

    // TODO: Add testing option that is pub(crate) and #cfg[test] that allows passing in custom boundary.
    #[test]
    fn should_upload_and_return_data_link() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "POST".into(),
            uri: "https://siasky.net/skynet/skyfile".into(),
            body: REQUEST_BODY.into(),
            headers: vec![("Content-Type".to_owned(), CONTENT_TYPE_MULTIPART.to_owned())],
            response: Some(RESPONSE_JSON.into()),
            sent: true,
            ..Default::default()
        });

        t.execute_with(|| {
            // Upload
            let skylink_returned = upload_bytes(&str_to_bytes(DATA), FILE_NAME, None).unwrap();

            // Check the response.
            assert_eq!(skylink_returned, str_to_bytes(EXPECTED_DATA_LINK));
        })
    }

    #[test]
    fn should_upload_with_custom_portal_url() {
        const CUSTOM_PORTAL_URL: &str = "https://siasky.dev";

        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "POST".into(),
            uri: "https://siasky.dev/skynet/skyfile".into(),
            body: REQUEST_BODY.into(),
            headers: vec![("Content-Type".to_owned(), CONTENT_TYPE_MULTIPART.to_owned())],
            response: Some(RESPONSE_JSON.into()),
            sent: true,
            ..Default::default()
        });

        t.execute_with(|| {
            // Upload
            let skylink_returned = upload_bytes(
                &str_to_bytes(DATA),
                FILE_NAME,
                Some(&UploadOptions {
                    common: CommonOptions {
                        portal_url: CUSTOM_PORTAL_URL,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            )
            .unwrap();

            // Check the response.
            assert_eq!(skylink_returned, str_to_bytes(EXPECTED_DATA_LINK));
        })
    }

    #[test]
    fn should_upload_with_custom_request_options() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "POST".into(),
            uri: "https://siasky.net/skynet/skyfile".into(),
            body: REQUEST_BODY.into(),
            headers: vec![
                ("Content-Type".to_owned(), CONTENT_TYPE_MULTIPART.to_owned()),
                ("Cookie".to_owned(), JWT_COOKIE.into()),
                ("Skynet-Api-Key".to_owned(), SKYNET_API_KEY.into()),
            ],
            response: Some(RESPONSE_JSON.into()),
            sent: true,
            ..Default::default()
        });

        t.execute_with(|| {
            // Upload
            let skylink_returned = upload_bytes(
                &str_to_bytes(DATA),
                FILE_NAME,
                Some(&UploadOptions {
                    common: CommonOptions {
                        custom_cookie: Some(JWT_COOKIE),
                        skynet_api_key: Some(SKYNET_API_KEY),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            )
            .unwrap();

            // Check the response.
            assert_eq!(skylink_returned, str_to_bytes(EXPECTED_DATA_LINK));
        })
    }
}
