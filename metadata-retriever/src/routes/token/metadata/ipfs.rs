use crate::routes::token::metadata::util::TryFromStr;
use cid::Cid;
use regex::Regex;
use url::Url;

pub const IPFS_GATEWAY: &str = "https://ipfs.io/ipfs/";
pub const CID_REGEX: &str = r"(Qm[1-9A-HJ-NP-Za-km-z]{44}|b[A-Za-z2-7]{58,}|B[A-Z2-7]{58,}|z[1-9A-HJ-NP-Za-km-z]{48,}|F[0-9A-F]{50,})";

#[derive(Debug, PartialEq, Clone)]
pub struct IpfsPath {
    cid: Cid,
    ext: Option<String>,
}

impl ToString for IpfsPath {
    fn to_string(&self) -> String {
        match &self.ext {
            Some(value) => format!("{}/{}", self.cid, value),
            None => self.cid.to_string(),
        }
    }
}

impl From<IpfsPath> for Url {
    fn from(path: IpfsPath) -> Self {
        Url::parse(&format!("{}{}", IPFS_GATEWAY, path.to_string()))
            .expect("IPFS objects can be transformed to Url")
    }
}
impl TryFromStr for IpfsPath {
    fn try_from_str(s: &str) -> Option<Self> {
        // Use regex pattern matching to attempt to extract a potential CID.
        let cid_re = Regex::new(CID_REGEX).unwrap();
        let captures = cid_re.captures(s)?;
        let cid_str = captures.get(0)?.as_str();
        let cid = cid_str.parse::<Cid>().ok()?;

        // Seek file path extension after the CID string detected
        // and forget everything before the CID string!
        let post_cid_re = Regex::new(&format!(
            r"{}[^/]*(?:/(?P<path>.+?))(?:[?#]|$)",
            regex::escape(cid_str)
        ))
        .unwrap();
        // Captures anything occurring after / beyond CID or None.
        let ext = match post_cid_re.captures(s) {
            Some(caps) => caps.get(1).map(|m| m.as_str().to_string()),
            None => None,
        };

        Some(Self { cid, ext })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn ipfs_cid_extraction() {
        let cid_v1 = "bafybeia737e3bpzusnxxn36alotv2fwviezjm44e5rx4fnuvmpfgcfh3ha";
        assert!(IpfsPath::try_from_str(cid_v1).is_some());
        let cid_v0 = "QmZxdRjNXwCdpMBzSHVTFownBREGxcFnhmw6D7FHomGYCF";
        assert!(IpfsPath::try_from_str(cid_v0).is_some());

        // This should not be considered a valid IPFS CID.
        let invalid_cid = "bVJoZEdFNmFXMWhaMlV2YzNabkszaHRiRHRpWVhObE5qUXNVRWhPTWxwNVFqTmhWMUl3WVVRd2JrMXFhM2RLZVVKdldsZHNibUZJVVRsS2VsVjNUVU5qWjJSdGJHeGtNRXAyWlVRd2JrMURRWGRKUkVrMVRVTkJNVTFFUVc";
        assert!(IpfsPath::try_from_str(invalid_cid).is_none());

        let real_examples = [
            "ipfs://Qmf2AR2YB4H32zL7muveWbs8GHp94udeAv5uZVX5wQ8WDL/2805",
            "ipfs://QmdXP2KNU2cuqcJBi6Uaf5bhnu2udmrbtJDfm3dMoewzNu/1193.json",
            "ipfs://QmUmwm3SEpJECJDypZyaaMVRAXzLc5dTjCwKskVCoZqFLA/metadata.json",
            "ipfs://bafybeicaxgijmayzrpk4uzevqwvp4icfcs2q46oxwcn4xh7bbml7kymari/1.json",
            // These two urls don't work:
            "https://cryptodesigns.mypinata.cloud/ipfs/Qmd2FrrBfZbzGdF1M2CNGkqxgWkzZK1odkAT82Lr4mbca6/1146.json",
            "https://forgotendogwtf.mypinata.cloud/ipfs/QmTf35rx8kbhNKk3nMdN3BCh2uHUxjFnBunYopBr4Lu5Fw/5071.json",
        ].map(IpfsPath::try_from_str);
        assert!(real_examples.iter().all(|x| x.is_some()));
    }

    #[tokio::test]
    async fn capturing_more() {
        // This test demonstrates, that although the URLs here are dead,
        // we can still extract the content from IPFS.
        let a = "https://cryptodesigns.mypinata.cloud/ipfs/Qmd2FrrBfZbzGdF1M2CNGkqxgWkzZK1odkAT82Lr4mbca6/1146.json";
        let b = "https://forgotendogwtf.mypinata.cloud/ipfs/QmTf35rx8kbhNKk3nMdN3BCh2uHUxjFnBunYopBr4Lu5Fw/5071.json";

        // as urls:
        let a_url = Url::parse(a).unwrap();
        let b_url = Url::parse(b).unwrap();
        let a_response = reqwest::get(a_url).await;
        let b_response = reqwest::get(b_url).await;
        assert!(a_response.is_err());
        assert_eq!(
            b_response.unwrap().text().await.unwrap(),
            "Account has been disabled. - ERR_ID:00022".to_string()
        );

        // as IPFS
        let apfs = IpfsPath::try_from_str(a).unwrap();
        let bpfs = IpfsPath::try_from_str(b).unwrap();
        let a_response = reqwest::get::<Url>(apfs.into()).await;
        let b_response = reqwest::get::<Url>(bpfs.into()).await;

        assert!(a_response.unwrap().json::<Value>().await.is_ok());
        assert!(b_response.unwrap().json::<Value>().await.is_ok());
    }

    #[test]
    fn valid_ipfs_capturing() {
        assert_eq!(
            &IpfsPath::try_from_str(
                "https://bafybeia737e3bpzusnxxn36alotv2fwviezjm44e5rx4fnuvmpfgcfh3ha.ipfs.nftstorage.link/40.json"
            ).unwrap().to_string(),
            "bafybeia737e3bpzusnxxn36alotv2fwviezjm44e5rx4fnuvmpfgcfh3ha/40.json"
        );

        assert_eq!(
            &IpfsPath::try_from_str(
                "https://cryptodesigns.mypinata.cloud/ipfs/Qmd2FrrBfZbzGdF1M2CNGkqxgWkzZK1odkAT82Lr4mbca6/1146.json"
            ).unwrap().to_string(),
            "Qmd2FrrBfZbzGdF1M2CNGkqxgWkzZK1odkAT82Lr4mbca6/1146.json",
            "crypto designs mypinata"
        );

        assert_eq!(
            &IpfsPath::try_from_str(
                "https://cloudflare-ipfs.com/ipfs/QmQVHMRMhVGqQH4vPDgxK2Y3rnToQSVbbhbyTq7qnVbgoA"
            )
            .unwrap()
            .to_string(),
            "QmQVHMRMhVGqQH4vPDgxK2Y3rnToQSVbbhbyTq7qnVbgoA",
            "cloudflare"
        );

        assert_eq!(
            &IpfsPath::try_from_str(
                "https://nearnaut.mypinata.cloud/ipfs/QmQVHMRMhVGqQH4vPDgxK2Y3rnToQSVbbhbyTq7qnVbgoA"
            ).unwrap().to_string(),
            "QmQVHMRMhVGqQH4vPDgxK2Y3rnToQSVbbhbyTq7qnVbgoA",
            "mypinata"
        );

        assert_eq!(
            &IpfsPath::try_from_str(
                "https://arweave.net//bafkreidmsup4r2r6quyjjo553zqyc5rupttmupgnv7k24opc3f4jbolq3a"
            )
            .unwrap()
            .to_string(),
            "bafkreidmsup4r2r6quyjjo553zqyc5rupttmupgnv7k24opc3f4jbolq3a",
            "arweave"
        );

        assert_eq!(
            &IpfsPath::try_from_str(
                "https://QmQVHMRMhVGqQH4vPDgxK2Y3rnToQSVbbhbyTq7qnVbgoA.ipfs.dweb.link"
            )
            .unwrap()
            .to_string(),
            "QmQVHMRMhVGqQH4vPDgxK2Y3rnToQSVbbhbyTq7qnVbgoA",
            "dweb.link"
        );

        assert_eq!(
            &IpfsPath::try_from_str(
                "https://ipfs.io/ipfs/bafybeifj7sronkwlpvtkcguq3rztzmr3lun5zoom63vpl2czqukejqbfky/0.png"
            ).unwrap().to_string(),
            "bafybeifj7sronkwlpvtkcguq3rztzmr3lun5zoom63vpl2czqukejqbfky/0.png",
            "trailing file"
        );

        assert_eq!(
            &IpfsPath::try_from_str(
                "https://ipfs.io/ipfs/bafybeig6ccro733era5le55xzezlq6ho7xab24kmxccwpds6igeqsqrrrm/output/mint/762.png"
            ).unwrap().to_string(),
            "bafybeig6ccro733era5le55xzezlq6ho7xab24kmxccwpds6igeqsqrrrm/output/mint/762.png",
            "trailing path"
        );

        assert_eq!(
            &IpfsPath::try_from_str(
                "https://ipfs.fleek.co/ipfs/QmZQV5YXKakh7aKqSk3MVARNu8eaxws9KNc6EeStQTYt5w"
            )
            .unwrap()
            .to_string(),
            "QmZQV5YXKakh7aKqSk3MVARNu8eaxws9KNc6EeStQTYt5w",
            "fleek"
        );

        assert_eq!(
            &IpfsPath::try_from_str(
                "https://ipfs.io/ipfs/bafybeifj7sronkwlpvtkcguq3rztzmr3lun5zoom63vpl2czqukejqbfky"
            )
            .unwrap()
            .to_string(),
            "bafybeifj7sronkwlpvtkcguq3rztzmr3lun5zoom63vpl2czqukejqbfky"
        );
    }
}
