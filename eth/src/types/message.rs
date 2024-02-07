use crate::types::{Address, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    Contract {
        address: Address,
    },
    Token {
        address: Address,
        token_id: U256,
        token_uri: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_round_trip() {
        let contract_struct = Message::Contract {
            address: Address::from(1),
        };
        let token_struct = Message::Token {
            address: Address::from(2),
            token_id: U256::from_dec_str("12345678999999").unwrap(),
            token_uri: Some("SupaString".into()),
        };

        // Serialize the struct to JSON
        let contract_json =
            serde_json::to_string(&contract_struct).expect("Failed to serialize to JSON");
        let token_json = serde_json::to_string(&token_struct).expect("Failed to serialize to JSON");

        // confirm json format of message payload
        assert_eq!(
            contract_json,
            r#"{"contract":{"address":"0x0000000000000000000000000000000000000001"}}"#
        );
        assert_eq!(
            token_json,
            r#"{"token":{"address":"0x0000000000000000000000000000000000000002","token_id":"12345678999999","token_uri":"SupaString"}}"#
        );

        let deserialized_contract_struct: Message =
            serde_json::from_str(&contract_json).expect("Failed to deserialize from JSON");
        let deserialized_token_struct: Message =
            serde_json::from_str(&token_json).expect("Failed to deserialize from JSON");

        assert_eq!(contract_struct, deserialized_contract_struct);
        assert_eq!(token_struct, deserialized_token_struct);
    }

    #[test]
    fn vector_serialization() {
        let contract_struct = Message::Contract {
            address: Address::from(1),
        };
        let token_struct = Message::Token {
            address: Address::from(2),
            token_id: U256::from_dec_str("12345678999999").unwrap(),
            token_uri: None,
        };
        let vec_request = vec![contract_struct, token_struct];
        let request_json =
            serde_json::to_string(&vec_request).expect("Failed to serialize to JSON");

        // confirm json format of message payload
        assert_eq!(
            request_json,
            r#"[{"contract":{"address":"0x0000000000000000000000000000000000000001"}},{"token":{"address":"0x0000000000000000000000000000000000000002","token_id":"12345678999999","token_uri":null}}]"#
        );

        // Deserialize the JSON back to the struct
        let deserialized_request_struct: Vec<Message> =
            serde_json::from_str(&request_json).expect("Failed to deserialize from JSON");

        assert_eq!(vec_request, deserialized_request_struct);
    }
}
