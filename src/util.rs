//! Utility functions.

use serde::{Deserialize, Deserializer, Serializer};
use sp_io::offchain;
use sp_runtime::offchain::{self as rt_offchain, http};
use sp_std::{str, vec::Vec};

/// The default Skynet portal URL.
pub const DEFAULT_PORTAL_URL: &str = "https://siasky.net";

/// The Skynet URI protocol prefix.
pub const URI_SKYNET_PREFIX: &str = "sia://";

/// Request error.
#[derive(Debug)]
pub enum RequestError {
    /// HTTP error.
    HttpError(rt_offchain::HttpError),
    /// HTTP error.
    HttpError2(http::Error),
    /// Timeout error.
    TimeoutError,
    /// Unexpected status.
    UnexpectedStatus(u16),
}

impl From<http::Error> for RequestError {
    fn from(err: http::Error) -> Self {
        Self::HttpError2(err)
    }
}

impl From<rt_offchain::HttpError> for RequestError {
    fn from(err: rt_offchain::HttpError) -> Self {
        Self::HttpError(err)
    }
}

pub fn concat_bytes(byte_slices: &[&[u8]]) -> Vec<u8> {
    let mut len = 0;
    for bytes in byte_slices {
        len += bytes.len();
    }
    let mut final_bytes = Vec::with_capacity(len);

    for bytes in byte_slices {
        let mut v = bytes.to_vec();
        final_bytes.append(&mut v);
    }

    final_bytes
}

pub fn concat_strs(strs: &[&str]) -> Vec<u8> {
    let mut len = 0;
    for s in strs {
        len += s.len();
    }
    let mut str_bytes = Vec::with_capacity(len);

    for s in strs {
        let mut v = str_to_bytes(s);
        str_bytes.append(&mut v);
    }

    str_bytes
}

pub fn execute_get(url: &str, custom_cookie: Option<&str>) -> Result<http::Response, RequestError> {
    // Initiate an external HTTP GET request. This is using high-level wrappers from `sp_runtime`.
    let mut request = http::Request::get(url);

    if let Some(cookie) = custom_cookie {
        request = request.add_header("Cookie", cookie);
    }

    execute_request(&request)
}

pub fn execute_request(request: &http::Request) -> Result<http::Response, RequestError> {
    // Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
    let timeout = offchain::timestamp().add(rt_offchain::Duration::from_millis(3000));

    let pending = request
        .clone()
        .deadline(timeout) // Setting the timeout time
        .send()?; // Sending the request out by the host

    // By default, the http request is async from the runtime perspective. So we are asking the
    // runtime to wait here. The returning value here is a `Result` of `Result`, so we are
    // unwrapping it twice by two `?`
    //
    // ref: https://substrate.dev/rustdocs/v2.0.0-rc3/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
    let response = pending
        .try_wait(timeout)
        .map_err(|_| RequestError::TimeoutError)??;

    if response.code >= 400 {
        Err(RequestError::UnexpectedStatus(response.code))
    } else {
        Ok(response)
    }
}

pub fn format_skylink(skylink: &[u8]) -> Vec<u8> {
    concat_bytes(&[&str_to_bytes(URI_SKYNET_PREFIX), skylink])
}

pub fn make_url(strs: &[&str]) -> Vec<u8> {
    let mut len = 0;
    for s in strs {
        len += s.len();

        // Add 1 for every slash that will be added to the URL later.
        if !s.ends_with('/') {
            len += 1;
        }
    }
    let mut url_bytes = Vec::with_capacity(len);

    let mut i = 0;
    for s in strs {
        // Remove any slashes from the beginning.
        let mut j = 0;
        while s[j..].starts_with('/') {
            j += 1;
        }
        let trimmed_s = &s[j..];

        if trimmed_s.is_empty() {
            i += 1;
            continue;
        }

        // Append the URL component to the URL.
        let mut v = str_to_bytes(trimmed_s);
        url_bytes.append(&mut v);

        // If this is not the last url component and there is no trailing slash, add one.
        if i < strs.len() - 1 && !trimmed_s.ends_with('/') {
            let mut slash = str_to_bytes("/");
            url_bytes.append(&mut slash);
        }

        i += 1;
    }

    url_bytes
}

pub fn str_to_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(de)?;
    Ok(s.as_bytes().to_vec())
}

pub fn ser_bytes_to_string<S>(v: &[u8], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(str::from_utf8(v).unwrap())
}

#[cfg(test)]
mod tests {
    use super::{make_url, str_to_bytes, DEFAULT_PORTAL_URL};
    use sp_std::str;

    #[test]
    fn make_url_test() {
        const ENTRY_LINK: &str = "AQAZ1R-KcL4NO_xIVf0q8B1ngPVd6ec-Pu54O0Cto387Nw";
        const EXPECTED_URL: &str =
            "https://siasky.net/AQAZ1R-KcL4NO_xIVf0q8B1ngPVd6ec-Pu54O0Cto387Nw";

        let url = make_url(&[DEFAULT_PORTAL_URL, "/", ENTRY_LINK]);
        assert_eq!(url, str_to_bytes(EXPECTED_URL));

        let url = make_url(&[DEFAULT_PORTAL_URL, ENTRY_LINK]);
        assert_eq!(url, str_to_bytes(EXPECTED_URL));
    }

    #[test]
    fn str_to_bytes_test() {
        const TEST_STR: &str = "foos";

        assert_eq!(TEST_STR, str::from_utf8(&str_to_bytes(TEST_STR)).unwrap());
    }
}
