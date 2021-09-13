use sp_std::str;

pub const DEFAULT_PORTAL_URL: &str = "https://siasky.net";

pub const URI_SKYNET_PREFIX: &str = "sia://";

pub fn concat_strs(strs: &[&str]) -> Vec<u8> {
    let mut len = 0;
    for s in strs {
        len += s.len();
    }
    let mut url_bytes = Vec::with_capacity(len);

    for s in strs {
        let mut v = str_to_bytes(s);
        url_bytes.append(&mut v);
    }

    url_bytes
}

// TODO: Make sure arguments are separated by "/".
pub fn make_url(args: &[&str]) -> Vec<u8> {
    concat_strs(args)
}

fn str_to_bytes(s: &str) -> Vec<u8> {
	s.as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_PORTAL_URL, make_url, str_to_bytes};
    use sp_std::str;

    #[test]
    fn make_url_test() {
        const ENTRY_LINK: &str = "AQAZ1R-KcL4NO_xIVf0q8B1ngPVd6ec-Pu54O0Cto387Nw";
        const EXPECTED_URL: &str = "https://siasky.net/AQAZ1R-KcL4NO_xIVf0q8B1ngPVd6ec-Pu54O0Cto387Nw";

        let url = make_url(&[DEFAULT_PORTAL_URL, "/", ENTRY_LINK]);
        assert_eq!(url, str_to_bytes(EXPECTED_URL));
    }

    #[test]
    fn str_to_bytes_test() {
        const TEST_STR: &str = "foos";

        assert_eq!(TEST_STR, str::from_utf8(&str_to_bytes(TEST_STR)).unwrap());
    }
}
