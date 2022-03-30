use sp_io::offchain;
use sp_runtime::offchain::{self as rt_offchain, http};

/// The default Skynet portal URL.
pub const DEFAULT_PORTAL_URL: &str = "https://siasky.net";

/// Options common to all methods.
#[derive(Debug)]
pub struct CommonOptions<'a> {
    /// The portal URL.
    pub portal_url: &'a str,
    /// Optional custom cookie.
    pub custom_cookie: Option<&'a str>,
    /// Optional Skynet API key.
    pub skynet_api_key: Option<&'a str>,
}

impl Default for CommonOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            custom_cookie: None,
            skynet_api_key: None,
        }
    }
}

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

pub fn add_headers<'a>(
    mut request: http::Request<'a>,
    common: &CommonOptions,
) -> http::Request<'a> {
    if let Some(cookie) = common.custom_cookie {
        request = request.add_header("Cookie", cookie);
    }
    if let Some(key) = common.skynet_api_key {
        request = request.add_header("Skynet-Api-Key", key);
    }

    request
}

pub fn execute_get(
    url: &str,
    common_options: &CommonOptions,
) -> Result<http::Response, RequestError> {
    // Initiate an external HTTP GET request. This is using high-level wrappers from `sp_runtime`.
    let mut request = http::Request::get(url);

    request = add_headers(request, common_options);

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
