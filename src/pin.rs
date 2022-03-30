//! Pin functions.

use crate::request::{execute_get, CommonOptions, RequestError};
use crate::util::{make_url, str_to_bytes, URI_SKYNET_PREFIX};

use sp_std::{str, vec::Vec};

/// Pin error.
#[derive(Debug)]
pub enum PinError {
    /// Request error.
    RequestError(RequestError),
    /// UTF8 error.
    Utf8Error(str::Utf8Error),
    /// Validation error.
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

/// Pin options.
#[derive(Debug)]
pub struct PinOptions<'a> {
    /// Common options.
    pub common: CommonOptions<'a>,
    /// The endpoint to contact.
    pub endpoint_pin: &'a str,
}

impl Default for PinOptions<'_> {
    fn default() -> Self {
        Self {
            common: Default::default(),
            endpoint_pin: "/skynet/pin",
        }
    }
}

/// Re-pins the given `skylink`.
pub fn pin_skylink(skylink: &str, opts: Option<&PinOptions>) -> Result<Vec<u8>, PinError> {
    let default = Default::default();
    let opts = opts.unwrap_or(&default);

    // TODO: Implement full skylink parsing.
    let skylink = if let Some(stripped) = skylink.strip_prefix(URI_SKYNET_PREFIX) {
        stripped
    } else {
        skylink
    };

    let url = make_url(&[opts.common.portal_url, opts.endpoint_pin, skylink]);

    let mut response = execute_get(str::from_utf8(&url)?, &opts.common)?;

    let headers = response.headers();
    let skylink = headers
        .find("skynet-skylink")
        .or_else(|| headers.find("Skynet-Skylink"));

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
    use sp_io::TestExternalities;

    const DATA_LINK: &str = "MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ";
    // const ENTRY_LINK: &str = "AQAZ1R-KcL4NO_xIVf0q8B1ngPVd6ec-Pu54O0Cto387Nw";

    #[test]
    fn should_pin_data_link() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: "https://siasky.net/skynet/pin/MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ"
                .into(),
            response: Some(br#""#.to_vec()),
            response_headers: vec![("Skynet-Skylink".to_owned(), DATA_LINK.to_owned())],
            sent: true,
            ..Default::default()
        });

        t.execute_with(|| {
            // Call pin_skylink.
            let skylink_returned = pin_skylink(DATA_LINK, None).unwrap();

            // Check the response.
            assert_eq!(skylink_returned, str_to_bytes(DATA_LINK));
        })
    }

    // // TODO: Update test.
    // #[test]
    // fn should_fail_to_pin_entry_link() {
    //     let (offchain, state) = testing::TestOffchainExt::new();
    //     let mut t = TestExternalities::default();
    //     t.register_extension(OffchainExt::new(offchain));

    //     // Add expected request.
    //     state.write().expect_request(testing::PendingRequest {
    //         method: "GET".into(),
    //         uri: "https://siasky.net/skynet/pin/MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ"
    //             .into(),
    //         response: Some(br#""#.to_vec()),
    //         response_headers: vec![("Skynet-Skylink".to_owned(), DATA_LINK.to_owned())],
    //         sent: true,
    //         ..Default::default()
    //     });

    //     t.execute_with(|| {
    //         // Call pin_skylink.
    //         let skylink_returned = pin_skylink(ENTRY_LINK, &Default::default()).unwrap();
    //     })
    // }
}
