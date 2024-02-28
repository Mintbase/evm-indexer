use anyhow::anyhow;
use url::Url;

pub const IPFS_GATEWAY: &str = "https://ipfs.io/ipfs/";

pub(crate) const ENS_URI: &str =
    "https://metadata.ens.domains/mainnet/0x57f1887a8bf19b14fc0df6fd9b2acc9af147ea85";
/// Returns an HTTP url for an IPFS object.
pub fn http_link_ipfs(url: Url) -> anyhow::Result<Url> {
    Url::parse(IPFS_GATEWAY)
        .unwrap()
        .join(
            url.to_string()
                .trim_start_matches("ipfs://")
                .trim_start_matches("ipfs/"),
        )
        .map_err(|e| anyhow!(e.to_string()))
}

pub trait TryFromStr: Sized {
    fn try_from_str(s: &str) -> Option<Self>;
}
