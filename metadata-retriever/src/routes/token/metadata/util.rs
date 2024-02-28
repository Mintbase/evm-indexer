pub(crate) const ENS_URI: &str =
    "https://metadata.ens.domains/mainnet/0x57f1887a8bf19b14fc0df6fd9b2acc9af147ea85";

pub trait TryFromStr: Sized {
    fn try_from_str(s: &str) -> Option<Self>;
}
