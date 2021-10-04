//! Skynet Substrate-compatible SDK.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
// TODO: Uncomment
// #![deny(missing_docs)]

mod crypto;
mod download;
mod pin;
mod registry;
mod upload;
mod util;

pub use crypto::{Signature, HASH_LENGTH, PUBLIC_KEY_LENGTH, PRIVATE_KEY_LENGTH, SIGNATURE_LENGTH};
pub use download::{download_bytes, DownloadError, DownloadOptions};
pub use pin::{pin_skylink, PinError};
pub use registry::{
    get_entry, set_entry, GetEntryError, GetEntryOptions, RegistryEntry, SetEntryError,
    SetEntryOptions, SignedRegistryEntry,
};
pub use upload::{upload_bytes, UploadError, UploadOptions};
pub use util::{RequestError, DEFAULT_PORTAL_URL, URI_SKYNET_PREFIX};
