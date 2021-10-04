//! Registry functions.

use crate::crypto::{Signature};
use crate::util::{execute_request, make_url, RequestError, DEFAULT_PORTAL_URL};

use sp_std::{prelude::Vec, str};

#[derive(Debug)]
pub enum GetEntryError {
    RequestError(RequestError),
    Utf8Error(str::Utf8Error),
}

#[derive(Debug)]
pub enum SetEntryError {
    RequestError(RequestError),
    Utf8Error(str::Utf8Error),
}

#[derive(Debug)]
pub struct GetEntryOptions<'a> {
    pub portal_url: &'a str,
    pub endpoint_get_entry: &'a str,
}

impl Default for GetEntryOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_get_entry: "/skynet/registry",
        }
    }
}

#[derive(Debug)]
pub struct SetEntryOptions<'a> {
    pub portal_url: &'a str,
    pub endpoint_set_entry: &'a str,
}

impl Default for SetEntryOptions<'_> {
    fn default() -> Self {
        Self {
            portal_url: DEFAULT_PORTAL_URL,
            endpoint_set_entry: "/skynet/registry",
        }
    }
}

/// Registry entry.
pub struct RegistryEntry {
    /// The key of the data for the given entry.
    dataKey: Vec<u8>,
    /// The data stored in the entry.
    data: Vec<u8>,
    /// The revision number for the entry.
    revision: u64,
}

/// Signed registry entry.
pub struct SignedRegistryEntry {
/// The signature of the registry entry.
  entry: Option<RegistryEntry>,
/// The registry entry.
  signature: Option<Signature>,
}

pub fn get_entry(
    public_key: &str,
    data_key: &str,
    opts: Option<&GetEntryOptions>,
) -> Result<SignedRegistryEntry, GetEntryError> {
}

pub fn set_entry(
    private_key: &str,
    entry: RegistryEntry,
    opts: Option<&SetEntryOptions>,
) -> Result<(), SetEntryError> {
}
