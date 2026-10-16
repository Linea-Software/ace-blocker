use nutype::nutype;
use xxhash_rust::xxh3::xxh3_64;

#[nutype(derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Display, AsRef))]
pub struct UriHash(u64);

impl UriHash {
    pub fn from_uri(uri: &http::uri::Uri) -> UriHash {
        UriHash::new(xxh3_64(uri.to_string().as_bytes()))
    }

    pub fn from_text(uri: &str) -> UriHash {
        UriHash::new(xxh3_64(uri.as_bytes()))
    }
}
