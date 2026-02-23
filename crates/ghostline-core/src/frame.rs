use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A single captured request/response pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    /// SHA-256 hash of the request (model + messages + params, excluding timestamps).
    pub request_hash: [u8; 32],
    /// Raw request bytes (MessagePack-encoded).
    pub request_bytes: Vec<u8>,
    /// Raw response bytes (MessagePack-encoded).
    pub response_bytes: Vec<u8>,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
    /// Unix timestamp (milliseconds) when the frame was captured.
    pub timestamp: u64,
}

impl Frame {
    /// Create a new frame, computing the request hash automatically.
    pub fn new(
        request_bytes: Vec<u8>,
        response_bytes: Vec<u8>,
        latency_ms: u64,
        timestamp: u64,
    ) -> Self {
        let request_hash = Self::hash_request(&request_bytes);
        Self {
            request_hash,
            request_bytes,
            response_bytes,
            latency_ms,
            timestamp,
        }
    }

    /// Compute SHA-256 hash of raw request bytes.
    pub fn hash_request(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Serialize this frame to MessagePack bytes.
    pub fn to_msgpack(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }

    /// Deserialize a frame from MessagePack bytes.
    pub fn from_msgpack(data: &[u8]) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_msgpack() {
        let frame = Frame::new(
            b"request data".to_vec(),
            b"response data".to_vec(),
            42,
            1700000000000,
        );
        let packed = frame.to_msgpack().unwrap();
        let unpacked = Frame::from_msgpack(&packed).unwrap();
        assert_eq!(frame.request_hash, unpacked.request_hash);
        assert_eq!(frame.request_bytes, unpacked.request_bytes);
        assert_eq!(frame.response_bytes, unpacked.response_bytes);
        assert_eq!(frame.latency_ms, unpacked.latency_ms);
        assert_eq!(frame.timestamp, unpacked.timestamp);
    }

    #[test]
    fn deterministic_hash() {
        let data = b"same input";
        let h1 = Frame::hash_request(data);
        let h2 = Frame::hash_request(data);
        assert_eq!(h1, h2);

        let h3 = Frame::hash_request(b"different input");
        assert_ne!(h1, h3);
    }
}
