//! Download functions.

use crate::request::{execute_get, CommonOptions, RequestError};
use crate::util::{make_url, URI_SKYNET_PREFIX};

use sp_std::{prelude::Vec, str};

/// Download error.
#[derive(Debug)]
pub enum DownloadError {
    /// Request error.
    RequestError(RequestError),
    /// UTF8 error.
    Utf8Error(str::Utf8Error),
}

impl From<RequestError> for DownloadError {
    fn from(err: RequestError) -> Self {
        Self::RequestError(err)
    }
}

impl From<str::Utf8Error> for DownloadError {
    fn from(err: str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

/// Download options.
#[derive(Debug)]
pub struct DownloadOptions<'a> {
    /// Common options.
    pub common: CommonOptions<'a>,
    /// The endpoint to contact.
    pub endpoint_download: &'a str,
}

impl Default for DownloadOptions<'_> {
    fn default() -> Self {
        Self {
            common: Default::default(),
            endpoint_download: "/",
        }
    }
}

/// Downloads the bytes at the given `skylink`.
pub fn download_bytes(
    skylink: &str,
    opts: Option<&DownloadOptions>,
) -> Result<Vec<u8>, DownloadError> {
    let default = Default::default();
    let opts = opts.unwrap_or(&default);

    // TODO: Implement full skylink parsing.
    let skylink = if let Some(stripped) = skylink.strip_prefix(URI_SKYNET_PREFIX) {
        stripped
    } else {
        skylink
    };

    let url = make_url(&[opts.common.portal_url, opts.endpoint_download, skylink]);

    let response = execute_get(str::from_utf8(&url)?, &opts.common)?;

    Ok(response.body().collect::<Vec<u8>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::str_to_bytes;

    use sp_core::offchain::{testing, OffchainWorkerExt};
    use sp_io::TestExternalities;

    const DATA_LINK: &str = "MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ";
    const EXPECTED_JSON: &str = "{ message: \"hi there!\" }";

    #[test]
    fn should_download_from_data_link() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: "https://siasky.net/MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ".into(),
            response: Some(str_to_bytes(EXPECTED_JSON)),
            response_headers: vec![("Skynet-Skylink".to_owned(), DATA_LINK.to_owned())],
            sent: true,
            ..Default::default()
        });

        t.execute_with(|| {
            // Download
            let data_returned = download_bytes(DATA_LINK, None).unwrap();

            // Check the response.
            assert_eq!(data_returned, str_to_bytes(EXPECTED_JSON));
        })
    }

    #[test]
    fn should_download_with_custom_portal_url() {
        const CUSTOM_PORTAL_URL: &str = "https://siasky.dev";

        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainWorkerExt::new(offchain));

        // Add expected request.
        state.write().expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: "https://siasky.dev/MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ".into(),
            response: Some(str_to_bytes(EXPECTED_JSON)),
            response_headers: vec![("Skynet-Skylink".to_owned(), DATA_LINK.to_owned())],
            sent: true,
            ..Default::default()
        });

        t.execute_with(|| {
            // Download
            let data_returned = download_bytes(
                DATA_LINK,
                Some(&DownloadOptions {
                    common: CommonOptions {
                        portal_url: CUSTOM_PORTAL_URL,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            )
            .unwrap();

            // Check the response.
            assert_eq!(data_returned, str_to_bytes(EXPECTED_JSON));
        })
    }
}
