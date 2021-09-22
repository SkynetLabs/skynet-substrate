//! Skynet Substrate-compatible SDK.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
// #![deny(missing_docs)]

mod download;
mod pin;
mod upload;
mod util;

pub use download::{download_bytes, DownloadError};
pub use pin::{pin_skylink, PinError};
pub use upload::{upload_bytes, UploadError};
pub use util::{RequestError, DEFAULT_PORTAL_URL, URI_SKYNET_PREFIX};
