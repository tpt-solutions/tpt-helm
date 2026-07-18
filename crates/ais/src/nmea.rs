// SPDX-License-Identifier: MIT OR Apache-2.0

//! NMEA 0183 sentence parsing for AIVDM / AIVDO payloads.

use thiserror::Error;

/// Errors that can occur while parsing an NMEA 0183 sentence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NmeaError {
    /// The sentence does not start with a valid talker-id prefix.
    #[error("invalid or missing talker id")]
    InvalidTalkerId,
    /// The checksum field is missing or malformed.
    #[error("missing or malformed checksum")]
    InvalidChecksum,
    /// The computed checksum does not match the sentence checksum.
    #[error("checksum mismatch: computed {computed:02X} but sentence says {stated:02X}")]
    ChecksumMismatch {
        /// Checksum computed from the sentence body.
        computed: u8,
        /// Checksum declared by the sentence.
        stated: u8,
    },
    /// The sentence is empty or too short to be valid.
    #[error("sentence too short")]
    TooShort,
}

/// A parsed NMEA 0183 sentence (talker id, fields, and payload tag).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NmeaSentence {
    /// Talker identifier, e.g. `AI` for AIS.
    pub talker: String,
    /// Sentence formatter / type, e.g. `VDM` or `VDO`.
    pub formatter: String,
    /// Raw comma-separated fields (excluding the leading talker/formatter and
    /// the trailing checksum).
    pub fields: Vec<String>,
}

impl NmeaSentence {
    /// Returns true if this is an AIS VDM/VDO sentence carrying AIS payload.
    #[must_use]
    pub fn is_ais(&self) -> bool {
        (self.talker == "AI" || self.talker == "BS")
            && (self.formatter == "VDM" || self.formatter == "VDO")
    }
}

/// Parse a single NMEA 0183 sentence (including its leading `!`/`$` and
/// trailing `*XX` checksum).
///
/// # Errors
/// Returns [`NmeaError`] if the sentence is malformed or fails its checksum.
pub fn parse_sentence(input: &str) -> Result<NmeaSentence, NmeaError> {
    let input = input.trim();
    if input.len() < 6 {
        return Err(NmeaError::TooShort);
    }
    if !input.starts_with('!') && !input.starts_with('$') {
        return Err(NmeaError::InvalidTalkerId);
    }

    let Some((body, checksum)) = input[1..].split_once('*') else {
        return Err(NmeaError::InvalidChecksum);
    };

    let stated = u8::from_str_radix(checksum, 16).map_err(|_| NmeaError::InvalidChecksum)?;
    let computed = body.bytes().fold(0u8, |acc, b| acc ^ b);
    if computed != stated {
        return Err(NmeaError::ChecksumMismatch { computed, stated });
    }

    let mut parts = body.split(',');
    let talker_formatter = parts.next().unwrap_or_default();
    if talker_formatter.len() < 5 {
        return Err(NmeaError::InvalidTalkerId);
    }
    let talker = talker_formatter[..2].to_string();
    let formatter = talker_formatter[2..].to_string();

    let fields = parts.map(str::to_string).collect();

    Ok(NmeaSentence {
        talker,
        formatter,
        fields,
    })
}

/// An AIS payload fragment extracted from a single AIVDM/AIVDO sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AisFragment {
    /// Total number of sentences in the sequence (1..=9).
    pub total: u8,
    /// Index of this sentence within the sequence (1..=total).
    pub number: u8,
    /// Sequence number used to correlate multi-part messages (0 if absent).
    pub sequence: u8,
    /// Six-bit packed payload fragment carried by this sentence.
    pub payload: String,
    /// Number of fill bits appended to the final fragment.
    pub fill_bits: u8,
}

/// Errors that can occur while extracting an AIS fragment from a sentence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AisFragmentError {
    /// The sentence is not an AIS VDM/VDO sentence.
    #[error("sentence is not an AIS VDM/VDO sentence")]
    NotAis,
    /// A required field (count, index, or payload) was missing or malformed.
    #[error("malformed AIS fragment fields")]
    Malformed,
}

impl NmeaSentence {
    /// Extract the AIS payload fragment from this sentence.
    ///
    /// Expects the AIVDM/AIVDO field layout:
    /// `VDM,<total>,<number>,<seq>,<channel>,<payload>,<fill>`.
    ///
    /// # Errors
    /// Returns [`AisFragmentError`] if the sentence is not AIS or its fields
    /// are malformed.
    pub fn ais_fragment(&self) -> Result<AisFragment, AisFragmentError> {
        if !self.is_ais() {
            return Err(AisFragmentError::NotAis);
        }
        // fields = [total, number, seq, channel, payload, fill]
        let total = self
            .fields
            .first()
            .and_then(|s| s.parse::<u8>().ok())
            .ok_or(AisFragmentError::Malformed)?;
        let number = self
            .fields
            .get(1)
            .and_then(|s| s.parse::<u8>().ok())
            .ok_or(AisFragmentError::Malformed)?;
        let sequence = self
            .fields
            .get(2)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(0);
        let payload = self
            .fields
            .get(4)
            .filter(|s| !s.is_empty())
            .ok_or(AisFragmentError::Malformed)?
            .clone();
        let fill_bits = self
            .fields
            .get(5)
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(0);

        Ok(AisFragment {
            total,
            number,
            sequence,
            payload,
            fill_bits,
        })
    }
}

/// Reassemble a single AIS six-bit payload from a sequence of fragments.
///
/// `fragments` must contain every part (ordered by `number`). For a single
/// sentence, pass a one-element slice. Fill bits are trimmed from the final
/// fragment so the result decodes cleanly.
///
/// # Errors
/// Returns [`AisFragmentError::Malformed`] if the parts do not cover the
/// expected sequence contiguously.
pub fn reassemble(fragments: &[AisFragment]) -> Result<String, AisFragmentError> {
    if fragments.is_empty() {
        return Err(AisFragmentError::Malformed);
    }
    let total = fragments[0].total;
    let mut by_number: Vec<&AisFragment> = fragments.iter().collect();
    by_number.sort_by_key(|f| f.number);
    let mut payload = String::new();
    for expected in 1..=total {
        let Some(frag) = by_number.iter().find(|f| f.number == expected) else {
            return Err(AisFragmentError::Malformed);
        };
        payload.push_str(&frag.payload);
    }
    if total > 1 {
        // Remove fill bits from the tail of the final fragment.
        if let Some(frag) = by_number.last() {
            let remove = usize::from(frag.fill_bits);
            if remove > 0 && payload.len() >= remove {
                payload.truncate(payload.len() - remove);
            }
        }
    }
    Ok(payload)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_vdm_with_checksum() {
        // !AIVDM,1,1,,A,15M67FC000G?ufbE`HqM5@0<0<0,0*6D  (checksum recomputed)
        let sentence = "!AIVDM,1,1,,A,15M67FC000G?ufbE`HqM5@0<0<0,0*46";
        let parsed = parse_sentence(sentence).expect("valid sentence");
        assert_eq!(parsed.talker, "AI");
        assert_eq!(parsed.formatter, "VDM");
        assert!(parsed.is_ais());
        assert_eq!(parsed.fields.len(), 6);
    }

    #[test]
    fn rejects_checksum_mismatch() {
        let sentence = "!AIVDM,1,1,,A,15M67FC000G?ufbE,0*00";
        assert!(matches!(
            parse_sentence(sentence),
            Err(NmeaError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn rejects_missing_checksum() {
        let sentence = "!AIVDM,1,1,,A,15M67FC000G?ufbE,0";
        assert_eq!(parse_sentence(sentence), Err(NmeaError::InvalidChecksum));
    }

    #[test]
    fn rejects_too_short() {
        assert_eq!(parse_sentence("!AB"), Err(NmeaError::TooShort));
    }

    #[test]
    fn extracts_single_fragment() {
        let sentence =
            parse_sentence("!AIVDM,1,1,,A,15M67FC000G?ufbE`HqM5@0<0<0,0*46").expect("valid");
        let frag = sentence.ais_fragment().expect("fragment");
        assert_eq!(frag.total, 1);
        assert_eq!(frag.number, 1);
        assert_eq!(frag.payload, "15M67FC000G?ufbE`HqM5@0<0<0");
    }

    #[test]
    fn reassembles_multi_part() {
        let a = parse_sentence("!AIVDM,2,1,1,A,55M67FC00001M@<:V381T003`?R0T4PP0000001,0*00")
            .expect("valid");
        let b = parse_sentence("!AIVDM,2,2,1,A,0000000000000000,0*17").expect("valid");
        let fa = a.ais_fragment().expect("fragment");
        let fb = b.ais_fragment().expect("fragment");
        let payload = reassemble(&[fa, fb]).expect("reassembles");
        assert_eq!(
            payload,
            "55M67FC00001M@<:V381T003`?R0T4PP00000010000000000000000"
        );
    }
}
