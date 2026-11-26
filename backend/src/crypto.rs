//! Envelope encryption (`AES-256-GCM`) for sensitive database secret values
//! (`users.totp_secret` and secret rows in `app_secrets`).
//!
//! When `SECRETS_ENCRYPTION_KEY` is configured (32 bytes as 64 hex chars or 44-char base64),
//! values are stored as `"enc:v1:" || base64(nonce_12 || ciphertext_with_tag)` bound via
//! GCM AAD to their record context (`"totp_secret:<user_id>"` or `"app_secret:<name>"`).
//! Reading supports transparent fallback for legacy plaintext rows until rotated.

use base64::Engine as _;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};

const PREFIX: &str = "enc:v1:";
const NONCE_LEN: usize = 12;

/// Parse a 32-byte encryption key from 64 hex chars or standard/URL base64.
pub fn parse_key_material(raw: &str) -> anyhow::Result<[u8; 32]> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("SECRETS_ENCRYPTION_KEY must not be empty");
    }
    if trimmed.len() == 64 && trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        let bytes = hex::decode(trimmed)?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("SECRETS_ENCRYPTION_KEY hex length mismatch"))?;
        return Ok(arr);
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(trimmed))
        .map_err(|_| {
            anyhow::anyhow!("SECRETS_ENCRYPTION_KEY must be 64 hex chars or 32-byte base64")
        })?;
    let arr: [u8; 32] = decoded
        .try_into()
        .map_err(|_| anyhow::anyhow!("SECRETS_ENCRYPTION_KEY must decode to exactly 32 bytes"))?;
    Ok(arr)
}

pub fn load_configured_key() -> anyhow::Result<Option<[u8; 32]>> {
    match std::env::var("SECRETS_ENCRYPTION_KEY") {
        Ok(raw) if !raw.trim().is_empty() => Ok(Some(parse_key_material(&raw)?)),
        _ => Ok(None),
    }
}

pub fn validate_configured_key() -> anyhow::Result<()> {
    let _ = load_configured_key()?;
    Ok(())
}

pub fn encrypt_with_key(
    key_bytes: &[u8; 32],
    context: &str,
    plaintext: &str,
) -> anyhow::Result<String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }
    let mut nonce_bytes = [0u8; NONCE_LEN];
    SystemRandom::new()
        .fill(&mut nonce_bytes)
        .map_err(|_| anyhow::anyhow!("CSPRNG failed"))?;
    let unbound = UnboundKey::new(&AES_256_GCM, key_bytes)
        .map_err(|_| anyhow::anyhow!("Invalid AES-256-GCM key"))?;
    let key = LessSafeKey::new(unbound);
    let mut in_out = plaintext.as_bytes().to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce_bytes),
        Aad::from(context.as_bytes()),
        &mut in_out,
    )
    .map_err(|_| anyhow::anyhow!("Secret encryption failed"))?;
    let mut combined = Vec::with_capacity(NONCE_LEN + in_out.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&in_out);
    Ok(format!(
        "{PREFIX}{}",
        base64::engine::general_purpose::STANDARD.encode(combined)
    ))
}

pub fn decrypt_with_key(
    key_bytes: Option<&[u8; 32]>,
    context: &str,
    stored: &str,
) -> anyhow::Result<String> {
    let Some(encoded) = stored.strip_prefix(PREFIX) else {
        return Ok(stored.to_string());
    };
    let key_bytes = key_bytes.ok_or_else(|| {
        anyhow::anyhow!("Encrypted secret encountered but SECRETS_ENCRYPTION_KEY is not configured")
    })?;
    let mut buf = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|_| anyhow::anyhow!("Invalid encrypted secret encoding"))?;
    if buf.len() < NONCE_LEN + AES_256_GCM.tag_len() {
        anyhow::bail!("Truncated encrypted secret payload");
    }
    let nonce_bytes: [u8; NONCE_LEN] = buf[..NONCE_LEN]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid secret nonce"))?;
    let unbound = UnboundKey::new(&AES_256_GCM, key_bytes)
        .map_err(|_| anyhow::anyhow!("Invalid AES-256-GCM key"))?;
    let key = LessSafeKey::new(unbound);
    let plain = key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce_bytes),
            Aad::from(context.as_bytes()),
            &mut buf[NONCE_LEN..],
        )
        .map_err(|_| anyhow::anyhow!("Secret decryption authentication failed"))?;
    Ok(String::from_utf8(plain.to_vec())?)
}

pub fn encrypt_secret(context: &str, plaintext: &str) -> anyhow::Result<String> {
    let Some(key_bytes) = load_configured_key()? else {
        return Ok(plaintext.to_string());
    };
    encrypt_with_key(&key_bytes, context, plaintext)
}

pub fn decrypt_secret(context: &str, stored: &str) -> anyhow::Result<String> {
    if !stored.starts_with(PREFIX) {
        return Ok(stored.to_string());
    }
    let key_bytes = load_configured_key()?;
    decrypt_with_key(key_bytes.as_ref(), context, stored)
}

pub fn is_encrypted(stored: &str) -> bool {
    stored.starts_with(PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_context_binding() {
        let key =
            parse_key_material("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
                .unwrap();
        let enc = encrypt_with_key(&key, "totp_secret:42", "JBSWY3DPEHPK3PXP").unwrap();
        assert!(enc.starts_with("enc:v1:"));
        assert!(!enc.contains("JBSWY3DPEHPK3PXP"));

        let dec = decrypt_with_key(Some(&key), "totp_secret:42", &enc).unwrap();
        assert_eq!(dec, "JBSWY3DPEHPK3PXP");

        // Wrong context AAD must fail authentication.
        assert!(decrypt_with_key(Some(&key), "totp_secret:99", &enc).is_err());

        // Legacy plaintext rows remain readable during migration.
        assert_eq!(
            decrypt_with_key(Some(&key), "totp_secret:42", "LEGACYBASE32").unwrap(),
            "LEGACYBASE32"
        );
    }
}
