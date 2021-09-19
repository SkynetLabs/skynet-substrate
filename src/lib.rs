//! Skynet Substrate-compatible SDK.

#![cfg_attr(not(feature = "std"), no_std)]

#![forbid(unsafe_code)]
// #![deny(missing_docs)]

mod download;
mod pin;
mod upload;
mod util;

pub use download::{DownloadError, download_bytes};
pub use pin::{PinError, pin_skylink};
pub use upload::{UploadError, upload_bytes};
pub use util::{RequestError};
