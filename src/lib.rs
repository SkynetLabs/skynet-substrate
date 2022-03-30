//! Skynet Substrate-compatible SDK.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

// TODO: Consolidate all error types into a single crate-wide error type.

mod crypto;
mod download;
mod encoding;
mod pin;
mod registry;
mod request;
mod skylink;
mod upload;
mod util;

pub use crypto::{Signature, HASH_LENGTH, PRIVATE_KEY_LENGTH, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};
pub use download::{download_bytes, DownloadError, DownloadOptions};
pub use pin::{pin_skylink, PinError};
pub use registry::{
    get_entry, get_entry_link, set_data_link, set_entry, set_entry_data, GetEntryError,
    GetEntryOptions, RegistryEntry, SetEntryError, SetEntryOptions, SignedRegistryEntry,
};
pub use request::{CommonOptions, RequestError, DEFAULT_PORTAL_URL};
pub use upload::{upload_bytes, UploadError, UploadOptions};
pub use util::URI_SKYNET_PREFIX;
