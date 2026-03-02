//! Usage: PKCE verifier/challenge generation for OAuth code flow.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub(crate) struct PkcePair {
    pub(crate) code_verifier: String,
    pub(crate) code_challenge: String,
}

pub(crate) fn generate_pkce_pair() -> PkcePair {
    let mut random = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut random);

    let code_verifier = URL_SAFE_NO_PAD.encode(random);
    let code_challenge = code_challenge_s256(&code_verifier);

    PkcePair {
        code_verifier,
        code_challenge,
    }
}

pub(crate) fn code_challenge_s256(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_pair_has_valid_lengths_and_consistent_challenge() {
        let pair = generate_pkce_pair();
        assert!(pair.code_verifier.len() >= 43);
        assert!(pair.code_verifier.len() <= 128);

        let expected = code_challenge_s256(&pair.code_verifier);
        assert_eq!(pair.code_challenge, expected);
    }
}
