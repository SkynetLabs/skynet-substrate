use ed25519_dalek;

pub type Signature = Vec<u8>;

pub const HASH_LENGTH: usize = 32;

pub const PUBLIC_KEY_LENGTH: usize = ed25519_dalek::PUBLIC_KEY_LENGTH * 2;

pub const PRIVATE_KEY_LENGTH: usize = ed25519_dalek::SECRET_KEY_LENGTH * 2;

pub const SIGNATURE_LENGTH: usize = ed25519_dalek::SIGNATURE_LENGTH;
