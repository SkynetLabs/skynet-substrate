//! Download functions.

use crate::util::{execute_request, make_url, DEFAULT_PORTAL_URL, RequestError, URI_SKYNET_PREFIX};

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

pub fn download_bytes(skylink: &str, opts: &DownloadOptions) -> Result<Vec<u8>, DownloadError> {
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
