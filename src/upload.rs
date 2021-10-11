//! Upload functions.

use crate::util::{concat_strs, de_string_to_bytes, format_skylink, make_url, DEFAULT_PORTAL_URL};

use serde::Deserialize;
use sp_io::offchain;
use sp_runtime::offchain::{self as rt_offchain, http};
use sp_std::{if_std, prelude::Vec, str};

const PORTAL_FILE_FIELD_NAME: &str = "file";

#[derive(Debug)]
pub enum UploadError {
    HttpError(rt_offchain::HttpError),
    HttpError2(http::Error),
    JsonError(serde_json::Error),
    TimeoutError,
    UnexpectedStatus(u16),
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

#[derive(Debug)]
pub struct UploadOptions<'a> {
    pub portal_url: &'a str,
    pub endpoint_upload: &'a str,

    pub custom_cookie: Option<&'a str>,
}

impl Default for UploadOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_upload: "/skynet/skyfile",
            custom_cookie: None,
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

pub fn upload_bytes(
    bytes: &str,
    filename: &str,
    opts: Option<&UploadOptions>,
) -> Result<Vec<u8>, UploadError> {
    let default = &Default::default();
    let opts = opts.unwrap_or(default);

    // Construct the URL.
    let url = make_url(&[opts.portal_url, opts.endpoint_upload]);

    // Build the request body boundary.

    let timestamp: u64 = offchain::timestamp().unix_millis();
    if_std! {
        println!("{:?}", timestamp);
    }

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

    let body = concat_strs(&[
        "--",
        str::from_utf8(&boundary)?,
        "\r\n",
        str::from_utf8(&headers)?,
        "\r\n",
        bytes,
        "\r\n",
        "--",
        str::from_utf8(&boundary)?,
        "--\r\n",
    ]);

    let content_type = concat_strs(&[
        "multipart/form-data; boundary=\"",
        str::from_utf8(&boundary)?,
        "\"",
    ]);

    // Initiate an external HTTP POST request. This is using high-level wrappers from `sp_runtime`.
    let mut request =
        rt_offchain::http::Request::post(str::from_utf8(&url)?, vec![body.as_slice()])
            .add_header("Content-Type", str::from_utf8(&content_type)?);

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

    use sp_core::offchain::{testing, OffchainExt};
    use sp_io::TestExternalities;

    const DATA_LINK: &str = "MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ";
    const EXPECTED_DATA_LINK: &str = "sia://MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ";
    const DATA: &str = "foo";
    const FILE_NAME: &str = "barfile";
    const REQUEST_BODY: &str = "--0000000000000000000000000000000000000000000000000000000000000000----\r\nContent-Disposition: form-data; name=\"file\"; filename=\"barfile\"\r\nContent-Type: application/octet-stream\r\n\r\nfoo\r\n--0000000000000000000000000000000000000000000000000000000000000000------\r\n";
    const RESPONSE_JSON: &str = "{\"skylink\": \"MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ\", \"merkleroot\": \"foo\", \"bitfield\": 1028}";
    const CONTENT_TYPE_MULTIPART: &str = "multipart/form-data; boundary=\"0000000000000000000000000000000000000000000000000000000000000000----\"";

    const JWT_COOKIE: &str = "MTYz...=="; // Don't use a full JWT as it's quite long.

    // TODO: Add testing option that is pub(crate) and #cfg[test] that allows passing in custom boundary.
    #[test]
    fn should_upload_and_return_data_link() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainExt::new(offchain));

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
            let skylink_returned = upload_bytes(DATA, FILE_NAME, None).unwrap();

            // Check the response.
            assert_eq!(skylink_returned, str_to_bytes(EXPECTED_DATA_LINK));
        })
    }

    #[test]
    fn should_upload_with_custom_portal_url() {
        const CUSTOM_PORTAL_URL: &str = "https://siasky.dev";

        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainExt::new(offchain));

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
                DATA,
                FILE_NAME,
                Some(&UploadOptions {
                    portal_url: CUSTOM_PORTAL_URL,
                    ..Default::default()
                }),
            )
            .unwrap();

            // Check the response.
            assert_eq!(skylink_returned, str_to_bytes(EXPECTED_DATA_LINK));
        })
    }

    #[test]
    fn should_upload_with_custom_cookie() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "POST".into(),
            uri: "https://siasky.net/skynet/skyfile".into(),
            body: REQUEST_BODY.into(),
            headers: vec![
                ("Content-Type".to_owned(), CONTENT_TYPE_MULTIPART.to_owned()),
                ("Cookie".to_owned(), JWT_COOKIE.into()),
            ],
            response: Some(RESPONSE_JSON.into()),
            sent: true,
            ..Default::default()
        });

        t.execute_with(|| {
            // Upload
            let skylink_returned = upload_bytes(
                DATA,
                FILE_NAME,
                Some(&UploadOptions {
                    custom_cookie: Some(JWT_COOKIE),
                    ..Default::default()
                }),
            )
            .unwrap();

            // Check the response.
            assert_eq!(skylink_returned, str_to_bytes(EXPECTED_DATA_LINK));
        })
    }
}
