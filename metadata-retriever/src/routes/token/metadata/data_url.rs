use crate::routes::token::metadata::{ipfs::IpfsPath, util::TryFromStr, FetchedMetadata};
use anyhow::Result;
use data_url::{forgiving_base64::InvalidBase64, DataUrl, DataUrlError};
use serde_json::{Error as SerdeError, Value};
use std::{error::Error, fmt, str::FromStr};
use url::Url;

fn sanitize_data_url(dirty_s: &str) -> String {
    dirty_s
        .replace(";utf8,", ";charset=utf8,")
        .replace('#', "%23")
}

impl FromStr for FetchedMetadata {
    type Err = DataUrlParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sanitized_s = sanitize_data_url(s);
        let raw = s.to_string();
        let data_url = DataUrl::process(&sanitized_s)?;
        let (body, _fragment) = data_url.decode_to_vec()?;

        let mime = data_url.mime_type();
        let hash = md5::compute(raw.as_bytes()).0.to_vec();

        match mime.type_.as_ref() {
            "application" if data_url.mime_type().subtype == "json" => Ok(Self {
                hash,
                raw: Some(raw),
                json: Some(serde_json::from_slice::<Value>(&body)?),
            }),
            "text" | "image" => {
                // TODO -- don't store raw image here in DB!
                Ok(Self {
                    hash,
                    raw: Some(raw),
                    json: None,
                })
            }
            _ => Err(DataUrlParseError::UnsupportedMimeType(
                data_url.mime_type().to_string(),
            )),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum UriType {
    Url(Url),
    Ipfs(IpfsPath),
    Data(String),
    InvalidUrl(String),
    Json(Value),
}

impl FromStr for UriType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Generic Check first for IPFS CID
        if let Some(path) = IpfsPath::try_from_str(s) {
            return Ok(Self::Ipfs(path));
        }
        // This catches IPFS data too - but we've already checked for it.
        match Url::parse(s) {
            Ok(mut url) => {
                if url.cannot_be_a_base() {
                    return Ok(Self::Data(s.to_string()));
                }
                if url.scheme() == "ar" {
                    // Could also, reset the scheme domain and path here:
                    url = Url::parse(&format!(
                        "https://arweave.net/{}{}",
                        url.domain().expect("arweave hash"),
                        url.path()
                    ))?;
                }
                Ok(Self::Url(url))
            }
            Err(err) => {
                // Try to parse as JSON:
                match serde_json::from_str::<Value>(s) {
                    Ok(value) => Ok(Self::Json(value)),
                    Err(_) => Ok(Self::InvalidUrl(err.to_string())),
                }
            }
        }
    }
}
#[derive(Debug)]
pub enum DataUrlParseError {
    DataUrlProcessing(DataUrlError),
    DecodeError(InvalidBase64),
    JsonParse(SerdeError),
    UnsupportedMimeType(String),
}

impl fmt::Display for DataUrlParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DataUrlParseError::DataUrlProcessing(ref err) => {
                write!(f, "Data URL processing error: {}", err)
            }
            DataUrlParseError::DecodeError(ref err) => {
                write!(f, "Data URL decoding error: {}", err)
            }
            DataUrlParseError::JsonParse(ref err) => write!(f, "JSON parse error: {}", err),
            DataUrlParseError::UnsupportedMimeType(ref err) => {
                write!(f, "Unsupported MIME type {}", err)
            }
        }
    }
}

impl Error for DataUrlParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            DataUrlParseError::DataUrlProcessing(ref err) => Some(err),
            DataUrlParseError::DecodeError(ref err) => Some(err),
            DataUrlParseError::JsonParse(ref err) => Some(err),
            DataUrlParseError::UnsupportedMimeType(_) => None,
        }
    }
}

// Implement From traits to automatically convert errors
impl From<DataUrlError> for DataUrlParseError {
    fn from(err: DataUrlError) -> DataUrlParseError {
        DataUrlParseError::DataUrlProcessing(err)
    }
}

impl From<InvalidBase64> for DataUrlParseError {
    fn from(err: InvalidBase64) -> DataUrlParseError {
        DataUrlParseError::DecodeError(err)
    }
}

impl From<SerdeError> for DataUrlParseError {
    fn from(err: SerdeError) -> DataUrlParseError {
        DataUrlParseError::JsonParse(err)
    }
}

impl From<String> for DataUrlParseError {
    fn from(err: String) -> DataUrlParseError {
        DataUrlParseError::UnsupportedMimeType(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    use url::Url;

    fn load_test_data(file_name: &str) -> String {
        let path = Path::new("test_data").join(file_name);
        fs::read_to_string(path).expect("failed to load test data")
    }

    #[allow(dead_code)]
    fn print_errors(uris: &[String]) {
        uris.iter()
            .map(|x| (x, FetchedMetadata::from_str(x)))
            .for_each(|(uri, data)| {
                if data.is_err() {
                    println!("Error with {uri}: {data:?}");
                }
            });
    }

    #[allow(dead_code)]
    fn print_results(uris: &[String]) {
        uris.iter()
            .map(|x| (x, FetchedMetadata::from_str(x)))
            .for_each(|(uri, data)| {
                println!("URI: {uri}:\n  {data:?}");
            });
    }
    #[test]
    fn valid_json_data_uris() {
        let uris = [
            load_test_data("valid_json_base64.txt"),
            load_test_data("valid_json_utf8.txt"),
            // This only works because we attempt to parse json in text fields.
            load_test_data("valid_json_plain.txt"),
        ];
        // print_results(&uris);
        assert!(uris
            .map(|x| FetchedMetadata::from_str(x.as_str()))
            .iter()
            .all(|x| x.is_ok()));
    }

    #[test]
    fn invalid_data_uris() {
        assert_eq!(
            FetchedMetadata::from_str(&load_test_data("invalid_json_base64.txt"))
                .unwrap_err()
                .to_string(),
            "JSON parse error: EOF while parsing a string at line 1 column 10"
        );

        assert_eq!(
            FetchedMetadata::from_str(&load_test_data("invalid_json_base64_2.txt"))
                .unwrap_err()
                .to_string(),
            "Data URL decoding error: lone alphabet symbol present"
        );

        assert_eq!(
            FetchedMetadata::from_str(
                "https://ipfs.io/ipfs/QmeP3a2bFdeTMNeyM9UcDswtJg7Y2PUDYmYLvscVCmLvqX"
            )
            .unwrap_err()
            .to_string(),
            "Data URL processing error: not a valid data url"
        );
    }

    #[test]
    fn valid_plain_text() {
        let test_string = load_test_data("valid_text_plain.txt");
        assert_eq!(
            FetchedMetadata::from_str(&test_string).unwrap(),
            FetchedMetadata {
                //  data:text/plain;charset=utf-8,☳☰☶☴☲%0A☷☲☰☴☰%0A☱☶☰☴☰%0A☵☰☲☴☶%0A☲☳☴☴☵%0A
                // ☳☰☶☴☲\n☷☲☰☴☰\n☱☶☰☴☰\n☵☰☲☴☶\n☲☳☴☴☵\n
                hash: vec![62, 11, 204, 74, 87, 10, 98, 180, 84, 249, 230, 164, 216, 54, 255, 45],
                raw: Some(test_string),
                json: None,
            }
        );
    }

    #[test]
    fn valid_svg_xml_formats() {
        // Its actually unclear if these are valid. They are Valid "Text",
        // But the image rendering has not been determined.
        let svg_xml = [
            load_test_data("valid_svg_utf8.txt"),
            load_test_data("valid_svg_utf8_2.txt"),
            load_test_data("valid_svg_base64.txt"),
        ];
        // print_results(&svg_xml);
        assert!(svg_xml
            .map(|x| FetchedMetadata::from_str(x.as_str()))
            .iter()
            .all(|x| x.is_ok()));
    }

    #[test]
    fn weird_other_stuff() {
        assert!(
            FetchedMetadata::from_str(&load_test_data("invalid_json_plain.txt"))
                .unwrap()
                .json
                .is_none()
        );
    }

    #[test]
    fn arweave_handling() {
        assert_eq!(
            UriType::from_str("ar://wEBkrd6fpeOCnimnE0TxYPP8Z9hdiPkQe1RwQNgLszk").unwrap(),
            UriType::Url(
                Url::parse("https://arweave.net/wEBkrd6fpeOCnimnE0TxYPP8Z9hdiPkQe1RwQNgLszk")
                    .unwrap()
            )
        );
        assert_eq!(
            UriType::from_str("ar://f1VFl6RQzco_hF1zsc_MvRYjW8b7B3PDdau0_YZPSZc/500").unwrap(),
            UriType::Url(
                Url::parse("https://arweave.net/f1VFl6RQzco_hF1zsc_MvRYjW8b7B3PDdau0_YZPSZc/500")
                    .unwrap()
            )
        );
        assert_eq!(
            UriType::from_str("ar://f1VFl6RQzco_hF1zsc_MvRYjW8b7B3PDdau0_YZPSZc/500.json").unwrap(),
            UriType::Url(
                Url::parse(
                    "https://arweave.net/f1VFl6RQzco_hF1zsc_MvRYjW8b7B3PDdau0_YZPSZc/500.json"
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn tiny_example() {
        let data = r#"data:application/json;utf8,{"name":"Good number 1"}"#;
        assert!(FetchedMetadata::from_str(data).is_ok());
        let data = r#"data:application/json;utf8,{"number":"number char #", "question_mark": "?"}"#;
        assert!(FetchedMetadata::from_str(data).is_ok());
    }

    #[test]
    fn json_uri() {
        let data = r#"{"name": "WHO404 NFT#1","external_url":"https://who404.wtf/"}"#;

        assert_eq!(
            UriType::from_str(data).unwrap(),
            UriType::Json(serde_json::from_str(data).unwrap())
        );
    }

    #[test]
    fn invalid_url() {
        assert_eq!(
            UriType::from_str("1234.json").unwrap(),
            UriType::InvalidUrl("relative URL without a base".into())
        );
    }

    #[test]
    fn test_data_parsing() {
        let data_uris = [
            r#"data:application/json;base64,eyJuYW1lIjogIkdBUyBpcyBjaGVhcCAgMS8xMjAwIiwgImRlc2NyaXB0aW9uIjogIkdBUyBpcyBjaGVhcCAiLCAiaW1hZ2UiOiAiaXBmczovL2JhZmtyZWlhajZjdDQ0ajVqbDdydzJqZjdicWcyNmFnc3Q2eHUycXZoanBvM3l5aTdhNnQzejZsYnN1P2lkPTEiLCAicHJvcGVydGllcyI6IHsibnVtYmVyIjogMSwgIm5hbWUiOiAiR0FTIGlzIGNoZWFwICJ9fQ=="#,
            r#"data:application/json;utf8,{"name": "%23000000", "description":%20"All%20Colors.%20Tokensized.%20Find%20your%20colors.%20Own%20your%20colors.%20The%20first%20meta-NFT%20for%20generative,%20on-chain%20art%20-%201/1.","external_url":%20"https://www.colorverse.io/000000", "image":%20"data:image/svg+xml;utf8,<svg%20xmlns='http://www.w3.org/2000/svg'><rect%20width='350'%20height='350'%20style='fill:%20%2523000000'><title>%2523000000</title></rect></svg>"}"#,
            r#"data:text/plain;charset=utf-8,☳☰☶☴☲%0A☷☲☰☴☰%0A☱☶☰☴☰%0A☵☰☲☴☶%0A☲☳☴☴☵%0A"#,
            r#"data:application/json,{"name":"SOLV Allocation Voucher #1 - 100000.00 - OneTime", "description":"Voucher #1 of SOLV allocation. Voucher is used to represent the lock-up allocations of a certain project, which is currently being used to trade in the OTC Market. Now, everyone can trade SOLV's allocations on Opensea or Solv Vouchers by trading the Voucher onchain!","image": "https://imgurl.solv.finance/meta/images/icSOLV/1.png","external_url":"https://app.solv.finance/icSOLV/1", "properties": {"owner":"0xb145ed57fdf5b6f4fc7d8ec9a2f03026a218f000","underlying":"0x256f2d67e52fe834726d2ddcd8413654f5eb8b53","underlyingSymbol":"SOLV","vestingAmount":"100000.000000000000000000","principal":"100000.000000000000000000","claimType":"OneTime","claimableAmount":"100000.000000000000000000","percentages":["100.00%"],"maturities":[1628380800]}}"#,
            r#"data:image/svg+xml;utf8,<svg width='350' height='350' xmlns='http://www.w3.org/2000/svg'><style>.mystyle { font-family: helvetica; font-size: 6px; fill: black; }</style><rect height='350' width='350' y='0' x='0' fill='white'/><rect x='11' y='11' width='82' height='82' style='fill:%23E67C32;'/><rect x='93' y='11' width='82' height='82' style='fill:%23FFFFFF;'/><rect x='175' y='11' width='82' height='82'  style='fill:%230000FF;'/><rect x='257' y='11' width='82' height='82' style='fill:%23AA00AA;'/><rect x='11' y='93' width='82' height='82' style='fill:%2355FF00;'/><rect x='93' y='93' width='82' height='82' style='fill:%23005555;'/><rect x='175' y='93' width='82' height='82' style='fill:%23AAFFFF;'/><rect x='257' y='93' width='82' height='82' style='fill:%23D631D8;'/><rect x='11' y='175' width='82' height='82' style='fill:%23300911;'/><rect x='93' y='175' width='82' height='82' style='fill:%23020908;'/><rect x='175' y='175' width='82' height='82' style='fill:%23181071;'/><rect x='257' y='175' width='82' height='82' style='fill:%23011269;'/><rect x='11' y='257' width='82' height='82' style='fill:%231ECC14;'/><rect x='93' y='257' width='82' height='82' style='fill:%231B82D6;'/><rect x='175' y='257' width='82' height='82' style='fill:%237FBAEA;'/><rect x='257' y='257' width='82' height='82' style='fill:%23DEF230;'/><text x='50%' y='98.5%' dominant-baseline='middle' text-anchor='middle' class='mystyle'>Colorverse Founder 0&%23160;&%23160;&%23160;&%23160;|&%23160;&%23160;&%23160;&%23160;0xfEe27FB71ae3FEEEf2c11f8e02037c42945E87C4&%23160;&%23160;&%23160;&%23160;|&%23160;&%23160;&%23160;&%23160;1/1&%23160;&%23160;&%23160;&%23160;|&%23160;&%23160;&%23160;&%23160;Block 11973759</text></svg>"#,
            r#"data:image/svg+xml;charset=utf-8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2036%2036%22%3E%3Cpath%20fill%3D%22%23AA8ED6%22%20d%3D%22M35.88%2011.83A9.87%209.87%200%200018%206.1%209.85%209.85%200%2000.38%2014.07C1.75%2022.6%2011.22%2031.57%2018%2034.03c6.78-2.46%2016.25-11.44%2017.62-19.96.17-.72.27-1.46.27-2.24z%22%2F%3E%3C%2Fsvg%3E"#,
            r#"data:text/plain,{"name":"Certificate of Growth 1", "description":"Seed Capital - Certificates of Growth", "image": "data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0nMS4wJyBlbmNvZGluZz0nVVRGLTgnPz48c3ZnIHhtbG5zPSdodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZycgeG1sbnM6eGxpbms9J2h0dHA6Ly93d3cudzMub3JnLzE5OTkveGxpbmsnIHg9JzBweCcgeT0nMHB4JyB3aWR0aD0nNDgwcHgnIGhlaWdodD0nNzQwcHgnIHZpZXdCb3g9JzAgMCA0ODAgNzQwJyBlbmFibGUtYmFja2dyb3VuZD0nbmV3IDAgMCA0ODAgNzQwJyB4bWw6c3BhY2U9J3ByZXNlcnZlJz48cmVjdCB5PScxNDAnIGZpbGw9JyMxNDE0MTQnIHdpZHRoPSc0ODAnIGhlaWdodD0nNTAwJy8+PGxpbmVhckdyYWRpZW50IGlkPSdTVkdJRF80XycgZ3JhZGllbnRVbml0cz0ndXNlclNwYWNlT25Vc2UnIHgxPSc0MDUnIHkxPSc1ODAnIHgyPSc0MDUnIHkyPScyMjAuMTE5MSc+IDxzdG9wICBvZmZzZXQ9JzAnIHN0eWxlPSdzdG9wLWNvbG9yOiMwMEZGMDAnLz4gPHN0b3AgIG9mZnNldD0nMScgc3R5bGU9J3N0b3AtY29sb3I6I0MyMDAwQicvPiA8L2xpbmVhckdyYWRpZW50PiA8cmVjdCB4PSczOTAnIHk9JzIyMC4xMTknIGZpbGw9J3VybCgjU1ZHSURfNF8pJyB3aWR0aD0nMzAnIGhlaWdodD0nMzU5Ljg4MScvPjxsaW5lYXJHcmFkaWVudCBpZD0nU1ZHSURfNV8nIGdyYWRpZW50VW5pdHM9J3VzZXJTcGFjZU9uVXNlJyB4MT0nMzc1JyB5MT0nNTgwJyB4Mj0nMzc1JyB5Mj0nMjIwLjExOTEnPiA8c3RvcCAgb2Zmc2V0PScwJyBzdHlsZT0nc3RvcC1jb2xvcjojMTQxNDE0Jy8+IDxzdG9wICBvZmZzZXQ9JzEnIHN0eWxlPSdzdG9wLWNvbG9yOiMwMEEwQzYnLz4gPC9saW5lYXJHcmFkaWVudD48cGF0aCBvcGFjaXR5PScwLjI1JyBmaWxsPSdub25lJyBzdHJva2U9JyNFQkVCRUInIHN0cm9rZS13aWR0aD0nMicgc3Ryb2tlLW1pdGVybGltaXQ9JzEwJyBkPSdNMzAwLDIyMHYzNjAgTTI0MCwyMjB2MzYwIE0xODAsMjIwIHYzNjAgTTEyMCwyMjB2MzYwIE02MCwyMjB2MzYwIE0zNjAsNTIwSDAgTTM2MCw0NjBIMCBNMzYwLDQwMEgwIE0zNjAsMzQwSDAgTTM2MCwyODBIMCcvPjxyZWN0IHg9JzM2MCcgeT0nMjYwJyBvcGFjaXR5PScwLjI1JyBmaWxsPScjRUJFQkVCJyB3aWR0aD0nMzAnIGhlaWdodD0nMTIwJy8+PHJlY3QgeD0nMzkwJyB5PScyNDAnIG9wYWNpdHk9JzAuMjUnIGZpbGw9JyNFQkVCRUInIHdpZHRoPSczMCcgaGVpZ2h0PScxNjAnLz48cmVjdCB4PSczNjAnIHk9JzIyMC4xMTknIGZpbGw9J3VybCgjU1ZHSURfNV8pJyB3aWR0aD0nMzAnIGhlaWdodD0nMzU5Ljg4MScvPjxsaW5lYXJHcmFkaWVudCBpZD0nU1ZHSURfMV8nIGdyYWRpZW50VW5pdHM9J3VzZXJTcGFjZU9uVXNlJyB4MT0nMTgwLjAwMDUnIHkxPSc2NTAnIHgyPScxODAuMDAwNScgeTI9JzE2MC4wMDA1Jz4gPHN0b3AgIG9mZnNldD0nMCcgc3R5bGU9J3N0b3AtY29sb3I6IzUyZjk5O3N0b3Atb3BhY2l0eTowJy8+PHN0b3AgIG9mZnNldD0nMC41JyBzdHlsZT0nc3RvcC1jb2xvcjojNTJmOTknLz4gPHN0b3AgIG9mZnNldD0nMScgc3R5bGU9J3N0b3AtY29sb3I6IzUyZjk5O3N0b3Atb3BhY2l0eTowJy8+PC9saW5lYXJHcmFkaWVudD4gPHJlY3QgeT0nMTYwJyBmaWxsPSd1cmwoI1NWR0lEXzFfKScgd2lkdGg9JzM2MCcgaGVpZ2h0PSc0ODAnLz48bGluZWFyR3JhZGllbnQgaWQ9J1NWR0lEXzJfJyBncmFkaWVudFVuaXRzPSd1c2VyU3BhY2VPblVzZScgeDE9JzAnIHkxPSc0MDAnIHgyPSczMzYnIHkyPSc0MDAnPjxzdG9wICBvZmZzZXQ9JzAnIHN0eWxlPSdzdG9wLWNvbG9yOiM5YjA2MDQ7c3RvcC1vcGFjaXR5OjAnLz48c3RvcCAgb2Zmc2V0PScwLjUnIHN0eWxlPSdzdG9wLWNvbG9yOiM5YjA2MDQnLz4gPHN0b3AgIG9mZnNldD0nMScgc3R5bGU9J3N0b3AtY29sb3I6IzliMDYwNDtzdG9wLW9wYWNpdHk6MCcvPjwvbGluZWFyR3JhZGllbnQ+IDxyZWN0IHk9JzE2MCcgZmlsbD0ndXJsKCNTVkdJRF8yXyknIHdpZHRoPSczNjAnIGhlaWdodD0nNDgwJy8+PGxpbmVhckdyYWRpZW50IGlkPSdTVkdJRF8zXycgZ3JhZGllbnRVbml0cz0ndXNlclNwYWNlT25Vc2UnIHgxPScyMjAuMDIyNScgeTE9JzMyOS40NDUzJyB4Mj0nMjc2JyB5Mj0nMjE5LjQ0NScgZ3JhZGllbnRUcmFuc2Zvcm09J21hdHJpeCgzLjYgMCAwIDMuNiAtNjg0LjA3NjIgLTYwNiknPiA8c3RvcCAgb2Zmc2V0PScwJyBzdHlsZT0nc3RvcC1jb2xvcjpoc2woMTU4LDEwMCUsNzUlKTtzdG9wLW9wYWNpdHk6MCcvPiA8c3RvcCAgb2Zmc2V0PScxJyBzdHlsZT0nc3RvcC1jb2xvcjpoc2woMTU4LDEwMCUsNzUlKScvPiA8L2xpbmVhckdyYWRpZW50PiA8cmVjdCB5PScyMjAnIGZpbGw9J3VybCgjU1ZHSURfM18pJyB3aWR0aD0nMzYwLjAwMScgaGVpZ2h0PSczNjAnLz48cGF0aCBmaWxsPScjRUJFQkVCJyBkPSdNMCw1ODB2MTYwaDQ4MFY1ODBIMHonLz4gPHBhdGggZmlsbD0nI0VCRUJFQicgZD0nTTAsMHYyMjBoNDgwVjBIMHonLz48cGF0aCBvcGFjaXR5PScwLjI1JyBmaWxsPSdub25lJyBzdHJva2U9JyNFQkVCRUInIHN0cm9rZS13aWR0aD0nMicgc3Ryb2tlLW1pdGVybGltaXQ9JzEwJyBkPSdNMzAwLDIyMHYzNjAgTTI0MCwyMjB2MzYwIE0xODAsMjIwdjM2MCBNMTIwLDIyMHYzNjAgTTYwLDIyMHYzNjAgTTM2MCw1MjBIMCBNMzYwLDQ2MEgwIE0zNjAsNDAwSDAgTTM2MCwzNDBIMCBNMzYwLDI4MEgwJy8+PHRleHQgdHJhbnNmb3JtPSdtYXRyaXgoMSAwIDAgMSAxMCAxODUuMjA2MSknIGZvbnQtZmFtaWx5PSdBcmlhbCcgZm9udC1zaXplPScxNic+U29pbCBNb2lzdHVyZSAoeC1heGlzKTwvdGV4dD48dGV4dCB0cmFuc2Zvcm09J21hdHJpeCgxIDAgMCAxIDEwIDE0MS40NTYxKScgZm9udC1mYW1pbHk9J0FyaWFsJyBmb250LXNpemU9JzE2Jz5UaW1lPC90ZXh0Pjx0ZXh0IHRyYW5zZm9ybT0nbWF0cml4KDEgMCAwIDEgMTAgNjA1LjIwNjEpJyBmb250LWZhbWlseT0nQXJpYWwnIGZvbnQtc2l6ZT0nMTYnPlBsYW50PC90ZXh0Pjx0ZXh0IHRyYW5zZm9ybT0nbWF0cml4KDEgMCAwIDEgMTAgNjUxLjQ1NjEpJyBmb250LWZhbWlseT0nQXJpYWwnIGZvbnQtc2l6ZT0nMTYnPkxvY2F0aW9uPC90ZXh0Pjx0ZXh0IHRyYW5zZm9ybT0nbWF0cml4KDEgMCAwIDEgMjUwIDE4NS4yMDYxKScgZm9udC1mYW1pbHk9J0FyaWFsJyBmb250LXNpemU9JzE2Jz5UZW1wZXJhdHVyZSAoeS1heGlzKTwvdGV4dD48dGV4dCB0cmFuc2Zvcm09J21hdHJpeCgxIDAgMCAxIDQyNy4wMDI5IDI0MS40NTU2KScgZmlsbD0nI0VCRUJFQicgZm9udC1mYW1pbHk9J0FyaWFsJyBmb250LXNpemU9JzE2Jz50ZXJyYTA8L3RleHQ+PHRleHQgdHJhbnNmb3JtPSdtYXRyaXgoMSAwIDAgMSA5IDgwLjczNDkpJyBmaWxsPScjMTQxNDE0JyBmb250LWZhbWlseT0nVGltZXMtUm9tYW4sIFRpbWVzJyBmb250LXNpemU9JzQ1Jz5DZXJ0aWZpY2F0ZSBvZiBHcm93dGggPC90ZXh0Pjx0ZXh0IHRyYW5zZm9ybT0nbWF0cml4KDEgMCAwIDEgMjQ5IDIwNS4zMzY0KScgZmlsbD0nIzE0MTQxNCcgZm9udC1mYW1pbHk9J0NvdXJpZXIsIG1vbm9zcGFjZScgZm9udC1zaXplPScyNCc+MjEsMEM8L3RleHQ+PHRleHQgdHJhbnNmb3JtPSdtYXRyaXgoMSAwIDAgMSA5IDIwNS4zMzU5KScgZmlsbD0nIzE0MTQxNCcgZm9udC1mYW1pbHk9J0NvdXJpZXIsIG1vbm9zcGFjZScgZm9udC1zaXplPScyNCc+NzQsNDYlPC90ZXh0Pjx0ZXh0IHRyYW5zZm9ybT0nbWF0cml4KDEgMCAwIDEgMTAgNjcxLjU4NTkpJz48dHNwYW4geD0nMCcgeT0nMCcgZmlsbD0nIzE0MTQxNCcgZm9udC1mYW1pbHk9J0NvdXJpZXIsIG1vbm9zcGFjZScgZm9udC1zaXplPScyNCc+dGVycmEwIHN0dWRpbzwvdHNwYW4+PHRzcGFuIHg9JzAnIHk9JzIwJyBmaWxsPScjMTQxNDE0JyBmb250LWZhbWlseT0nQ291cmllcicgZm9udC1zaXplPScyNCc+dGVycmEwPC90c3Bhbj48L3RleHQ+PHRleHQgdHJhbnNmb3JtPSdtYXRyaXgoMSAwIDAgMSAxMCA2MjUuMzM1OSknIGZpbGw9JyMxNDE0MTQnIGZvbnQtZmFtaWx5PSdDb3VyaWVyLCBtb25vc3BhY2UnIGZvbnQtc2l6ZT0nMjQnPkR5cHNpcyBsdXRlc2NlbnM8L3RleHQ+PHRleHQgdHJhbnNmb3JtPSdtYXRyaXgoMSAwIDAgMSA0MTIuMzkwNiA4MCknIGZpbGw9JyMxNDE0MTQnIGZvbnQtZmFtaWx5PSdDb3VyaWVyLCBtb25vc3BhY2UnIGZvbnQtc2l6ZT0nMjQnPjE8L3RleHQ+PHRleHQgdHJhbnNmb3JtPSdtYXRyaXgoMSAwIDAgMSAxMCAxNjEuNTg1OSknIGZpbGw9JyMxNDE0MTQnIGZvbnQtZmFtaWx5PSdDb3VyaWVyLCBtb25vc3BhY2UnIGZvbnQtc2l6ZT0nMjQnPk1vbiwgMDcgTWFyIDIwMjIgMTg6MTU6MzAgR01UPC90ZXh0Pjx0ZXh0IHRyYW5zZm9ybT0nbWF0cml4KDEgMCAwIDEgMTQ3IDQxMyknIGZpbGw9JyNFQkVCRUInIGZvbnQtZmFtaWx5PSdDb3VyaWVyLCBtb25vc3BhY2UnIGZvbnQtc2l6ZT0nNDAnPis8L3RleHQ+PHRleHQgdHJhbnNmb3JtPSdtYXRyaXgoMSAwIDAgMSAzNjAgMzM5KScgZmlsbD0nI0VCRUJFQicgZm9udC1mYW1pbHk9J0NvdXJpZXIsIG1vbm9zcGFjZScgZm9udC1zaXplPSc0OSc+XzwvdGV4dD48dGV4dCB0cmFuc2Zvcm09J21hdHJpeCgxIDAgMCAxIDM5MCAzMzApJyBmaWxsPScjRUJFQkVCJyBmb250LWZhbWlseT0nQ291cmllciwgbW9ub3NwYWNlJyBmb250LXNpemU9JzQ5Jz5fPC90ZXh0Pjwvc3ZnID4=","attributes": [{"trait_type":"Venue","value":"terra0 studio"},{"trait_type":"Curator","value":"terra0"}]}"#,
            r#"data:text/plain,{"name":"Japan","description":"","created_by":"@the_innerspace","attributes":[{"trait_type":"Level","value":0},{"trait_type": "State","value":"Not playing"},{"trait_type": "Opponent","value":"None"}],"image":"<svg version='1.1' width='600' height='600' xmlns='http://www.w3.org/2000/svg' viewBox='0 0 600 600'> <style> .xo { font: normal 123px Andale Mono, monospace; fill: hsl(292,100%,49%); } .bg { fill: hsl(292,100%,98%); } .fg { stroke: hsl(292,100%,41%); } @keyframes pulse { from { stroke: hsl(292,100%,41%); } to { stroke: hsl(292,100%,98%); } } .xoline { stroke-width: 10; animation-iteration-count: infinite; animation-direction: alternate; animation-name: pulse; animation-duration: 1s; animation-timing-function: ease-in; } .tieStroke { stroke: transparent; } </style> <defs> <filter id='f1' x='0' y='0' width='200%' height='200%'> <feOffset result='offOut' in='SourceAlpha' dx='15' dy='15' /> <feBlend in='SourceGraphic' in2='offOut' mode='normal' /> </filter> <g id='o'> <rect class='xo' width='200' height='200' /> <circle class='xoline' cx='98' cy='98' stroke='white' fill='transparent' stroke-width='4' r='90' /> </g> <g id='x'> <rect class='xo' width='200' height='200' /> <path class='xoline' d='M 0 0 L 200 200 M 200 0 L 0 200' stroke='white' stroke-width='4' /> </g> <filter id='glow' x='-10%' y='-50%' with='200%' height='200%'> <feGaussianBlur in='SourceGraphic' stdDeviation='5'> <animate attributeName='stdDeviation' from='0' to='10' dur='2s' repeatCount='indefinite' values='1; 10; 5; 1;' /> </feGaussianBlur> </filter> </defs> <rect width='100%' height='100%' class='bg fg' stroke-width='20'> <animate id='strobo' attributeName='' values='transparent; hsl(0, 100%, 100%); hsl(0, 100%, 0%);' dur='200ms' repeatCount='indefinite'/> <animate id='psychedelic' attributeName='' values='#B500D1;#4500AD;#00BFE6;#008F07;#FFD900;#FF8C00;#F50010;#B500D1;' dur='1s' repeatCount='indefinite'/> </rect>  <path visibility='' d='M 0 200 H 600 M 00 400 H 600 M 200 0 V 600 M 400 00 V 600' stroke-width='10' stroke-linejoin='round' class='fg' /> <g visibility='hidden' class='rainbowTie'> <rect y='-600' height='86' fill='#B500D1' width='600'/> <rect y='-516' height='86' fill='#4500AD' width='600'/> <rect y='-430' height='86' fill='#00BFE6' width='600'/> <rect y='-344' height='86' fill='#008F07' width='600'/> <rect y='-258' height='86' fill='#FFD900' width='600'/> <rect y='-172' height='86' fill='#FF8C00' width='600'/> <rect y='-86' height='86' fill='#F50010' width='600'/> <rect y='0' height='86' fill='#B500D1' width='600'/> <rect y='86' height='86' fill='#4500AD' width='600'/> <rect y='172' height='86' fill='#00BFE6' width='600'/> <rect y='258' height='86' fill='#008F07' width='600'/> <rect y='344' height='86' fill='#FFD900' width='600'/> <rect y='430' height='86' fill='#FF8C00' width='600'/> <rect y='516' height='86' fill='#F50010' width='600'/> <animateTransform attributeName='transform' attributeType='XML' type='translate' from='0 0' to='0 600' dur='5s' repeatCount='indefinite'/> </g> <g id='tie' visibility='hidden'> <mask id='tieMask'> <path d='M 280 80 L 250 480 L 300 530 L 350 480 L 320 80 L 325 60 L 300 50 L 275 60 L 280 80 L 320 80' fill='white' stroke-linejoin='round' /> </mask> <path d='M 280 80 L 250 480 L 300 530 L 350 480 L 320 80 L 325 60 L 300 50 L 275 60 L 280 80 L 320 80' stroke-linejoin='round' class='bg' /> <path visibility='hidden' d='M 280 80 L 250 480 L 300 530 L 350 480 L 320 80 L 325 60 L 300 50 L 275 60 L 280 80 L 320 80' stroke-linejoin='round' stroke-width='10' filter='url(#glow)' class='tieStroke' /> <g mask='url(#tieMask)'> <rect x='250' y='100' height='15' fill='#B500D1' width='200' transform='rotate(20)' /> <rect x='250' y='115' height='15' fill='#4500AD' width='200' transform='rotate(20)' /> <rect x='250' y='130' height='15' fill='#00BFE6' width='200' transform='rotate(20)' /> <rect x='250' y='145' height='15' fill='#008F07' width='200' transform='rotate(20)' /> <rect x='250' y='160' height='15' fill='#FFD900' width='200' transform='rotate(20)' /> <rect x='250' y='175' height='15' fill='#FF8C00' width='200' transform='rotate(20)' /> <rect x='250' y='190' height='15' fill='#F50010' width='200' transform='rotate(20)' /> </g> <path d='M 280 80 L 250 480 L 300 530 L 350 480 L 320 80 L 325 60 L 300 50 L 275 60 L 280 80 L 320 80' fill='transparent' stroke-width='10' stroke-linejoin='round' class='tieStroke'> <animate id='strobo-tie' attributeName='' values='transparent; hsl(0, 100%, 100%); hsl(0, 100%, 0%);' dur='200ms' repeatCount='indefinite'/> </path> </g> </svg>"}"#,
            r#"data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iVVRGLTgiPz48c3ZnIHZpZXdCb3g9IjAgMCA1MDAgNTAwIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHN0eWxlPSJiYWNrZ3JvdW5kLWNvbG9yOiMwMDAwMDAiPjxwb2x5Z29uIHBvaW50cz0iNDAwLDEwMCA0MDAsNDAwIDEwMCw0MDAiICBmaWxsPSIjNjlmZjM3IiAvPjwvc3ZnPg=="#,
        ];

        for d in data_uris {
            match Url::parse(d) {
                Ok(url) => {
                    assert_eq!(url.scheme(), "data");
                    assert!(url.cannot_be_a_base());
                    // rest of content is in path.
                    // let x: Vec<&str> = url.path().split(';').collect();
                    // println!("Path Left {}", x[0]);
                }
                Err(err) => panic!("Error - {err:?}"),
            }
        }
    }
}
