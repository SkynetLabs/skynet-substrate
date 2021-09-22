//! Download functions.

use crate::util::{execute_request, make_url, RequestError, DEFAULT_PORTAL_URL, URI_SKYNET_PREFIX};

use sp_std::{prelude::Vec, str};

#[derive(Debug)]
pub enum DownloadError {
    RequestError(RequestError),
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

    let url = make_url(&[opts.portal_url, opts.endpoint_download, skylink]);

    let response = execute_request(str::from_utf8(&url)?)?;

    Ok(response.body().collect::<Vec<u8>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::str_to_bytes;

    use sp_core::offchain::{testing, OffchainExt};
    use sp_io::TestExternalities;

    const DATA_LINK: &str = "MABdWWku6YETM2zooGCjQi26Rs4a6Hb74q26i-vMMcximQ";
    const EXPECTED_JSON: &str = "{ message: \"hi there!\" }";

    #[test]
    fn should_download_from_data_link() {
        let (offchain, state) = testing::TestOffchainExt::new();
        let mut t = TestExternalities::default();
        t.register_extension(OffchainExt::new(offchain));

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
        t.register_extension(OffchainExt::new(offchain));

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
                    portal_url: CUSTOM_PORTAL_URL,
                    ..Default::default()
                }),
            )
            .unwrap();

            // Check the response.
            assert_eq!(data_returned, str_to_bytes(EXPECTED_JSON));
        })
    }
}
