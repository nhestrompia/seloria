use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Serialize to deterministic bincode bytes
pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CoreError> {
    bincode::serialize(value).map_err(|e| CoreError::Serialization(e.to_string()))
}

/// Deserialize from bincode bytes
pub fn from_bytes<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, CoreError> {
    bincode::deserialize(bytes).map_err(|e| CoreError::Deserialization(e.to_string()))
}

/// Serialize to JSON string (for RPC)
pub fn to_json<T: Serialize>(value: &T) -> Result<String, CoreError> {
    serde_json::to_string(value).map_err(|e| CoreError::Serialization(e.to_string()))
}

/// Serialize to pretty JSON string
pub fn to_json_pretty<T: Serialize>(value: &T) -> Result<String, CoreError> {
    serde_json::to_string_pretty(value).map_err(|e| CoreError::Serialization(e.to_string()))
}

/// Deserialize from JSON string
pub fn from_json<'a, T: Deserialize<'a>>(json: &'a str) -> Result<T, CoreError> {
    serde_json::from_str(json).map_err(|e| CoreError::Deserialization(e.to_string()))
}

/// Deserialize from JSON bytes
pub fn from_json_bytes<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, CoreError> {
    serde_json::from_slice(bytes).map_err(|e| CoreError::Deserialization(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        field1: u64,
        field2: String,
    }

    #[test]
    fn test_bincode_roundtrip() {
        let original = TestStruct {
            field1: 42,
            field2: "hello".to_string(),
        };

        let bytes = to_bytes(&original).unwrap();
        let recovered: TestStruct = from_bytes(&bytes).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_json_roundtrip() {
        let original = TestStruct {
            field1: 42,
            field2: "hello".to_string(),
        };

        let json = to_json(&original).unwrap();
        let recovered: TestStruct = from_json(&json).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_deterministic_serialization() {
        let value = TestStruct {
            field1: 100,
            field2: "test".to_string(),
        };

        let bytes1 = to_bytes(&value).unwrap();
        let bytes2 = to_bytes(&value).unwrap();
        assert_eq!(bytes1, bytes2);
    }
}
