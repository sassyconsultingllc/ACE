//! MCP Codec — Length-Prefixed Bincode Framing for tokio-util
//!
//! Implements a tokio-util Encoder/Decoder for framing MCP messages
//! over any AsyncRead/AsyncWrite transport (WebSocket, TCP, etc.).
//!
//! FRAME FORMAT:
//! ─────────────────────────────────────────────────────────────────────────
//! ┌──────────┬──────────────┬─────────────────────┐
//! │ Length   │ Magic        │ Payload (bincode)    │
//! │ 4 bytes  │ 4 bytes      │ variable             │
//! │ BE u32   │ "SASY"       │ McpCommand/Response  │
//! └──────────┴──────────────┴─────────────────────┘
//!
//! Length field = Magic.len() + Payload.len() (does NOT include itself)
//!
//! WHY:
//! - WebSocket messages can be fragmented
//! - TCP is a byte stream with no message boundaries
//! - Length prefix + magic provides clean framing + validation
//! - bincode payload is compact and fast

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::mcp_protocol::{ProtocolError, MAGIC};

/// Maximum frame size: 16 MB (generous for screenshots + page content)
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Codec for encoding/decoding length-prefixed bincode MCP frames.
///
/// Used with `tokio_util::codec::Framed` to wrap an AsyncRead/AsyncWrite:
/// ```ignore
/// let framed = Framed::new(transport, McpCodec::new());
/// ```
pub struct McpCodec {
    /// Maximum allowed frame size (configurable, default 16MB)
    max_frame_size: usize,
}

impl McpCodec {
    pub fn new() -> Self {
        Self {
            max_frame_size: MAX_FRAME_SIZE,
        }
    }

    pub fn with_max_frame_size(max_frame_size: usize) -> Self {
        Self { max_frame_size }
    }
}

impl Default for McpCodec {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// DECODER — Bytes → Frame
// ═══════════════════════════════════════════════════════════════════════════════

impl Decoder for McpCodec {
    type Item = BytesMut;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least 4 bytes for the length prefix
        if src.len() < 4 {
            return Ok(None);
        }

        // Peek at the length without consuming
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let frame_len = u32::from_be_bytes(length_bytes) as usize;

        // Validate frame size
        if frame_len > self.max_frame_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                ProtocolError::FrameTooLarge(frame_len),
            ));
        }

        // Check if we have the complete frame
        let total_len = 4 + frame_len; // length prefix + frame body
        if src.len() < total_len {
            // Reserve space for the rest of the frame
            src.reserve(total_len - src.len());
            return Ok(None);
        }

        // Consume the length prefix
        src.advance(4);

        // Split off the frame body
        let frame = src.split_to(frame_len);

        // Validate magic bytes
        if frame.len() < MAGIC.len() || &frame[..MAGIC.len()] != &MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                ProtocolError::BadMagic,
            ));
        }

        Ok(Some(frame))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ENCODER — Frame → Bytes
// ═══════════════════════════════════════════════════════════════════════════════

impl Encoder<Vec<u8>> for McpCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // item should already contain MAGIC + bincode payload
        if item.len() > self.max_frame_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                ProtocolError::FrameTooLarge(item.len()),
            ));
        }

        // Write length prefix (frame body length, not including the 4-byte prefix itself)
        let frame_len = item.len() as u32;
        dst.reserve(4 + item.len());
        dst.put_u32(frame_len);
        dst.extend_from_slice(&item);

        Ok(())
    }
}

/// Helper: Build a raw frame body (MAGIC + bincode payload) from serializable data.
/// Used by both client and server to create frame bodies for the encoder.
pub fn build_frame_body<T: serde::Serialize>(data: &T) -> Result<Vec<u8>, bincode::Error> {
    let payload = bincode::serialize(data)?;
    let mut body = Vec::with_capacity(MAGIC.len() + payload.len());
    body.extend_from_slice(&MAGIC);
    body.extend_from_slice(&payload);
    Ok(body)
}

/// Helper: Parse a frame body (MAGIC + bincode payload) into deserialized data.
/// Used by both client and server to parse frame bodies from the decoder.
pub fn parse_frame_body<T: serde::de::DeserializeOwned>(frame: &[u8]) -> Result<T, ProtocolError> {
    if frame.len() < MAGIC.len() {
        return Err(ProtocolError::TooShort);
    }
    if &frame[..MAGIC.len()] != &MAGIC {
        return Err(ProtocolError::BadMagic);
    }
    bincode::deserialize(&frame[MAGIC.len()..])
        .map_err(|e| ProtocolError::DeserializeError(e.to_string()))
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_protocol::{McpCommand, McpResponse};

    #[test]
    fn test_codec_roundtrip_command() {
        let mut codec = McpCodec::new();
        let cmd = McpCommand::Ping { seq: 99 };

        // Encode
        let body = build_frame_body(&cmd).unwrap();
        let mut buf = BytesMut::new();
        codec.encode(body, &mut buf).unwrap();

        // Decode
        let frame = codec.decode(&mut buf).unwrap().unwrap();
        let decoded: McpCommand = parse_frame_body(&frame).unwrap();

        match decoded {
            McpCommand::Ping { seq } => assert_eq!(seq, 99),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_codec_roundtrip_response() {
        let mut codec = McpCodec::new();
        let resp = McpResponse::Pong { seq: 42 };

        let body = build_frame_body(&resp).unwrap();
        let mut buf = BytesMut::new();
        codec.encode(body, &mut buf).unwrap();

        let frame = codec.decode(&mut buf).unwrap().unwrap();
        let decoded: McpResponse = parse_frame_body(&frame).unwrap();

        match decoded {
            McpResponse::Pong { seq } => assert_eq!(seq, 42),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_partial_frame_returns_none() {
        let mut codec = McpCodec::new();
        let cmd = McpCommand::GoBack;

        let body = build_frame_body(&cmd).unwrap();
        let mut buf = BytesMut::new();
        codec.encode(body, &mut buf).unwrap();

        // Only provide first 3 bytes — not enough for length prefix
        let mut partial = buf.split_to(3);
        assert!(codec.decode(&mut partial).unwrap().is_none());
    }

    #[test]
    fn test_oversized_frame_rejected() {
        let mut codec = McpCodec::with_max_frame_size(10); // Very small limit

        let mut buf = BytesMut::new();
        // Write a length prefix claiming 1000 bytes
        buf.put_u32(1000);
        buf.extend_from_slice(&[0u8; 1000]);

        assert!(codec.decode(&mut buf).is_err());
    }
}
