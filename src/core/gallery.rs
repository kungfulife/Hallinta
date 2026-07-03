use sha2::{Digest, Sha256};

pub fn verify_checksum(content: &str, expected_checksum: &str) -> bool {
    let expected_hex = expected_checksum
        .strip_prefix("sha256:")
        .unwrap_or(expected_checksum);
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    let computed_hex = hex_encode(result);
    computed_hex == expected_hex
}

pub fn compute_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{}", hex_encode(result))
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let bytes = bytes.as_ref();
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_round_trip_uses_stable_sha256_hex() {
        let checksum = compute_checksum("Hallinta");

        assert_eq!(
            checksum,
            "sha256:07e8ce079397dc37b1e5c7f4aef0dfe1ca71b68f6f9374ee40de064f225f1080"
        );
        assert!(verify_checksum("Hallinta", &checksum));
        assert!(verify_checksum(
            "Hallinta",
            checksum.trim_start_matches("sha256:")
        ));
        assert!(!verify_checksum("hallinta", &checksum));
    }
}
