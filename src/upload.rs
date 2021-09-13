//! Upload functions.

use crate::util::{concat_strs, make_url, DEFAULT_PORTAL_URL};

use frame_support::debug;
use sp_io::offchain;
use sp_runtime::offchain::{self as rt_offchain, http};
use sp_std::{if_std, prelude::Vec, str};

const PORTAL_FILE_FIELD_NAME: &str = "file";

#[derive(Debug)]
pub enum UploadError {
    HttpError(rt_offchain::HttpError),
    HttpError2(http::Error),
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

impl From<str::Utf8Error> for UploadError {
    fn from(err: str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

#[derive(Debug)]
pub struct UploadOptions<'a> {
    pub portal_url: &'a str,
    pub endpoint_upload: &'a str,
}

impl Default for UploadOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_upload: "/",
        }
    }
}

pub fn upload_bytes(
    bytes: &str,
    filename: &str,
    opts: &UploadOptions,
) -> Result<Vec<u8>, UploadError> {
    // Construct the URL.
    let url = make_url(&[opts.portal_url, opts.endpoint_upload]);

    if_std! {
        println!("{:?}", str::from_utf8(&url)?);
    }

    // Build the request body boundary.

    let timestamp: u64 = offchain::timestamp().unix_millis();

    // Make a 68-character boundary.
    // TODO: Use a random boundary? Wasn't sure how to do that in Substrate.
    let mut strs = Vec::<&str>::with_capacity(65);
    for i in 0..64 {
        strs[i] = if timestamp & (1 << (63 - i)) > 0 {
            "1"
        } else {
            "0"
        }
    }
    strs[64] = "----";
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
    let request = rt_offchain::http::Request::post(str::from_utf8(&url)?, vec![body.as_slice()])
        .add_header("Content-Type", str::from_utf8(&content_type)?);

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
        debug::error!("Unexpected http request status code: {}", response.code);
        return Err(UploadError::UnexpectedStatus(response.code));
    }

    // Next we fully read the response body and collect it to a vector of bytes.
    let body = response.body().collect::<Vec<u8>>();
    Ok(body)

    // let skylink = body.skylink;
    // Ok(concat_strs(&[URI_SKYNET_PREFIX, skylink]))
}
