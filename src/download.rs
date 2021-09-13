//! Download functions.

use crate::util::{make_url, DEFAULT_PORTAL_URL, URI_SKYNET_PREFIX};

use frame_support::debug;
use sp_io::offchain;
use sp_runtime::offchain::{self as rt_offchain, http};
use sp_std::{if_std, prelude::Vec, str};

#[derive(Debug)]
pub enum DownloadError {
    HttpError(rt_offchain::HttpError),
    HttpError2(http::Error),
    TimeoutError,
    UnexpectedStatus(u16),
    Utf8Error(str::Utf8Error),
}

impl From<http::Error> for DownloadError {
    fn from(err: http::Error) -> Self {
        Self::HttpError2(err)
    }
}

impl From<rt_offchain::HttpError> for DownloadError {
    fn from(err: rt_offchain::HttpError) -> Self {
        Self::HttpError(err)
    }
}

impl From<str::Utf8Error> for DownloadError {
    fn from(err: str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

#[derive(Debug)]
pub struct DownloadOptions<'a> {
    pub portal_url: &'a str,
    pub endpoint_download: &'a str,
}

impl Default for DownloadOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_download: "/",
        }
    }
}

pub fn download_bytes(skylink: &str, opts: &DownloadOptions) -> Result<Vec<u8>, DownloadError> {
    // TODO: Implement full skylink parsing.
    let skylink = if let Some(stripped) = skylink.strip_prefix(URI_SKYNET_PREFIX) {
        stripped
    } else {
        skylink
    };

    let url = make_url(&[opts.portal_url, opts.endpoint_download, skylink]);

    if_std! {
        println!("{:?}", str::from_utf8(&url)?);
    }

    // Initiate an external HTTP GET request. This is using high-level wrappers from `sp_runtime`.
    let request = rt_offchain::http::Request::get(str::from_utf8(&url)?);

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
        .map_err(|_| DownloadError::TimeoutError)??;

    if response.code >= 400 {
        debug::error!("Unexpected http request status code: {}", response.code);
        return Err(DownloadError::UnexpectedStatus(response.code));
    }

    // Next we fully read the response body and collect it to a vector of bytes.
    Ok(response.body().collect::<Vec<u8>>())
}

#[cfg(test)]
mod tests {
    use super::{download_bytes, DownloadOptions};
    use sp_std::if_std;

    const ENTRY_LINK: &str = "AQAZ1R-KcL4NO_xIVf0q8B1ngPVd6ec-Pu54O0Cto387Nw";
    const EXPECTED_JSON: &str = "{ message: \"hi there!\" }";

    #[test]
    fn download_bytes_entry_link() {
        let data = download_bytes(ENTRY_LINK, &Default::default());
        if_std! {
            println!("{:?}", data);
        }

        // TODO: Check the used portal url.
    }

    #[test]
    fn download_bytes_custom_portal_url() {
        const CUSTOM_PORTAL_URL: &str = "asdf";

        let _ = download_bytes(
            ENTRY_LINK,
            &DownloadOptions {
                portal_url: CUSTOM_PORTAL_URL,
                ..Default::default()
            },
        );

        // TODO: Check the used portal url.
    }
}
