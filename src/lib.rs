//! Skynet Substrate-compatible SDK.

#![cfg_attr(not(feature = "std"), no_std)]

mod download;
mod util;

pub use download::{DownloadError, get_file_content};
