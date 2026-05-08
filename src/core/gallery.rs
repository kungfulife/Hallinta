use sha2::{Digest, Sha256};

pub fn verify_checksum(content: &str, expected_checksum: &str) -> bool {
    let expected_hex = expected_checksum
        .strip_prefix("sha256:")
        .unwrap_or(expected_checksum);
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    let computed_hex = format!("{:x}", result);
    computed_hex == expected_hex
}

pub fn compute_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{:x}", result)
}
