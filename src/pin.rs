//! Pin functions.

use crate::util::{
    execute_request, make_url, str_to_bytes, RequestError, DEFAULT_PORTAL_URL, URI_SKYNET_PREFIX,
};

use sp_std::str;

#[derive(Debug)]
pub enum PinError {
    RequestError(RequestError),
    Utf8Error(str::Utf8Error),
    ValidationError(Vec<u8>),
}

impl From<RequestError> for PinError {
    fn from(err: RequestError) -> Self {
        Self::RequestError(err)
    }
}

impl From<str::Utf8Error> for PinError {
    fn from(err: str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

#[derive(Debug)]
pub struct PinOptions<'a> {
    pub portal_url: &'a str,
    pub endpoint_pin: &'a str,
}

impl Default for PinOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_pin: "/skynet/pin",
        }
    }
}

pub fn pin_skylink(skylink: &str, opts: &PinOptions) -> Result<Vec<u8>, PinError> {
    // TODO: Implement full skylink parsing.
    let skylink = if let Some(stripped) = skylink.strip_prefix(URI_SKYNET_PREFIX) {
        stripped
    } else {
        skylink
    };

    let url = make_url(&[opts.portal_url, opts.endpoint_pin, skylink]);

    let mut response = execute_request(str::from_utf8(&url)?)?;

    let headers = response.headers();
    let skylink = headers.find("skynet-skylink");

    if let Some(skylink) = skylink {
        Ok(str_to_bytes(skylink))
    } else {
        Err(PinError::ValidationError(str_to_bytes(
            "'skynet-skylink' header not found in response",
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::offchain::{testing, OffchainWorkerExt};
    use sp_runtime::offchain::{self as rt_offchain, http};
    use sp_io::TestExternalities;

    // TODO: Update test.
    #[test]
    fn should_send_a_basic_request_and_get_response() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        t.execute_with(|| {
            let request = http::Request::get("http://localhost:1234");
            let pending = request.add_header("X-Auth", "hunter2").send().unwrap();
            // make sure it's sent correctly
            state.write().fulfill_pending_request(
                0,
                testing::PendingRequest {
                    method: "GET".into(),
                    uri: "http://localhost:1234".into(),
                    headers: vec![("X-Auth".into(), "hunter2".into())],
                    sent: true,
                    ..Default::default()
                },
                b"1234".to_vec(),
                None,
            );

            // wait
            let mut response = pending.wait().unwrap();

            // then check the response
            let mut headers = response.headers().into_iter();
            assert_eq!(headers.current(), None);
            assert_eq!(headers.next(), false);
            assert_eq!(headers.current(), None);

            let body = response.body();
            assert_eq!(body.clone().collect::<Vec<_>>(), b"1234".to_vec());
            assert_eq!(body.error(), &None);
        })
    }
}
