//! Payload bytes: the one encode and the two decodes every verb shares.
//!
//! The encoding is protobuf, through [`prost`], and the SDK adds no framing of
//! its own. The envelope around a payload — the name, the raiser, the era of
//! the wire it travelled on — is the host's, so a version byte here would be a
//! second answer to a question already answered.

use crate::error::{Error, Refusal, Result};
use prost::Message;

/// One allocation, sized exactly: prost asks the message its encoded length
/// and fills a buffer of that length, so nothing downstream re-copies.
pub(crate) fn encode<T: Message>(value: &T) -> Vec<u8> {
    value.encode_to_vec()
}

pub(crate) fn decode<T: Message + Default>(bytes: &[u8]) -> Result<T> {
    T::decode(bytes).map_err(|cause| {
        Error::from_wire(
            0,
            format!("payload does not parse as the declared type: {cause}"),
            Vec::new(),
        )
    })
}

/// The decode a request handler applies to what arrived. A stale peer becomes a
/// typed refusal the caller reads, never a silently wrong value.
pub(crate) fn decode_refusing<T: Message + Default>(bytes: &[u8]) -> Result<T, Refusal> {
    decode(bytes).map_err(|e| Refusal::Unknown {
        message: e.message().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[derive(Clone, PartialEq, prost::Message)]
    struct Sample {
        #[prost(uint64, tag = "1")]
        who: u64,
        #[prost(uint32, tag = "2")]
        score: u32,
    }

    #[test]
    fn roundtrips_with_no_spare_capacity() {
        let value = Sample { who: 42, score: 9 };
        let bytes = encode(&value);
        assert_eq!(
            bytes.capacity(),
            bytes.len(),
            "spare capacity makes every crossing re-copy the payload"
        );
        let Ok(back) = decode::<Sample>(&bytes) else {
            panic!("what encode wrote has to decode");
        };
        assert_eq!(back, value);
    }

    #[test]
    fn a_payload_of_another_shape_refuses_by_name() {
        let Err(e) = decode::<Sample>(&[0xff, 0xff, 0xff]) else {
            panic!("garbage is not a Sample");
        };
        assert!(e.message().contains("declared type"));
    }

    /// Protobuf reads an empty payload as every field at its default, which is
    /// what lets a message gain a field without breaking a peer predating it.
    #[test]
    fn an_empty_payload_is_the_default_message() {
        let Ok(empty) = decode::<Sample>(&[]) else {
            panic!("an empty payload is legal protobuf");
        };
        assert_eq!(empty, Sample::default());
    }
}
