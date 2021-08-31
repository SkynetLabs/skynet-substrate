use sp_std::str;

pub const DEFAULT_PORTAL_URL: &str = "https://siasky.net";

pub const URI_SKYNET_PREFIX: &str = "sia://";

pub fn make_url(args: &[&str]) -> Vec<u8> {
    concat_strs(args)
}

fn str_to_bytes(s: &str) -> Vec<u8> {
	s.as_bytes().to_vec()
}

fn concat_strs(strs: &[&str]) -> Vec<u8> {
    let mut url_bytes = Vec::new();

    for s in strs {
        let mut v = str_to_bytes(s);
        url_bytes.append(&mut v);
    }

    url_bytes
}
