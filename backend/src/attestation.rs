//! Android Key Attestation X.509 chain & ASN.1 extension (`1.3.6.1.4.1.11129.2.1.17`) verifier.
//!
//! Used by `routes::auth::biometrics` (`AND-05`) to verify that enrolled P-256 biometric
//! signing keys are hardware-backed (`TrustedEnvironment` / `StrongBox`), biometric-gated
//! (`userAuthType` bit 2), challenge-bound, and bound to `org.pjstatecm.graceride`.

use chrono::{DateTime, NaiveDateTime, Utc};
use ring::signature;
use sha2::{Digest, Sha256};

pub const EXPECTED_ANDROID_PACKAGE: &str = "org.pjstatecm.graceride";

/// Official Google Android Hardware Attestation Root CA SPKI SHA-256 digests:
/// 1) Legacy & RSA-4096 Hardware Attestation Roots (`serialNumber=f92009e853b6b045`)
/// 2) RKP Key Attestation CA1 (`CN=Key Attestation CA1, OU=Android, O=Google LLC, C=US`)
pub const GOOGLE_ATTESTATION_ROOT_SPKI_SHA256: &[&str] = &[
    "feb2ea7551ee316ed4bb443c8293b884dbfdea40b603ee3e4f4a897e4580fbae",
    "3ee44512a1af2beb39c889490c60ea3f82e43f5d5a5532f5ab9419f676cd07ec",
];

// OID 1.3.6.1.4.1.11129.2.1.17 (Android Key Attestation KeyDescription)
const OID_ANDROID_ATTESTATION: &[u8] =
    &[0x2b, 0x06, 0x01, 0x04, 0x01, 0xd6, 0x79, 0x02, 0x01, 0x11];
// OID 2.5.29.19 (BasicConstraints)
const OID_BASIC_CONSTRAINTS: &[u8] = &[0x55, 0x1d, 0x13];
// OID 1.2.840.10045.2.1 (ecPublicKey)
const OID_EC_PUBLIC_KEY: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01];
// OID 1.2.840.10045.3.1.7 (prime256v1 / secp256r1)
const OID_PRIME256V1: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
// OID 1.3.132.0.34 (secp384r1)
const OID_SECP384R1: &[u8] = &[0x2b, 0x81, 0x04, 0x00, 0x22];
// OID 1.2.840.113549.1.1.1 (rsaEncryption)
const OID_RSA_ENCRYPTION: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
// Signature OIDs
const OID_SHA256_WITH_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
const OID_SHA384_WITH_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0c];
const OID_SHA512_WITH_RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0d];
const OID_ECDSA_WITH_SHA256: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02];
const OID_ECDSA_WITH_SHA384: &[u8] = &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x03];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DerTag {
    class: u8,
    constructed: bool,
    number: u32,
}

/// Read one ASN.1 DER TLV node, returning `(tag, full_encoded_node, value_slice, rest)`.
fn parse_tlv(input: &[u8]) -> anyhow::Result<(DerTag, &[u8], &[u8], &[u8])> {
    if input.is_empty() {
        anyhow::bail!("unexpected end of DER input");
    }
    let first = input[0];
    let class = (first >> 6) & 0x03;
    let constructed = (first & 0x20) != 0;
    let low = first & 0x1f;
    let mut pos = 1usize;
    let number = if low < 0x1f {
        u32::from(low)
    } else {
        let mut n = 0u32;
        loop {
            let b = *input
                .get(pos)
                .ok_or_else(|| anyhow::anyhow!("truncated DER tag"))?;
            pos += 1;
            if n == 0 && b == 0x80 {
                anyhow::bail!("non-canonical DER high tag");
            }
            n = n
                .checked_shl(7)
                .ok_or_else(|| anyhow::anyhow!("DER tag overflow"))?
                | u32::from(b & 0x7f);
            if (b & 0x80) == 0 {
                break n;
            }
        }
    };
    let len_b = *input
        .get(pos)
        .ok_or_else(|| anyhow::anyhow!("truncated DER length"))?;
    pos += 1;
    let len = if len_b < 0x80 {
        usize::from(len_b)
    } else {
        let num_bytes = usize::from(len_b & 0x7f);
        if num_bytes == 0 || num_bytes > 4 {
            anyhow::bail!("unsupported DER length encoding");
        }
        let len_slice = input
            .get(pos..pos + num_bytes)
            .ok_or_else(|| anyhow::anyhow!("truncated DER length bytes"))?;
        pos += num_bytes;
        if len_slice[0] == 0 {
            anyhow::bail!("non-canonical DER length");
        }
        let mut v = 0usize;
        for &b in len_slice {
            v = (v << 8) | usize::from(b);
        }
        if v < 0x80 {
            anyhow::bail!("non-canonical long DER length");
        }
        v
    };
    let end = pos
        .checked_add(len)
        .ok_or_else(|| anyhow::anyhow!("DER length overflow"))?;
    if end > input.len() {
        anyhow::bail!("truncated DER value");
    }
    Ok((
        DerTag {
            class,
            constructed,
            number,
        },
        &input[..end],
        &input[pos..end],
        &input[end..],
    ))
}

fn expect_universal(input: &[u8], num: u32) -> anyhow::Result<(&[u8], &[u8], &[u8])> {
    let (tag, whole, val, rest) = parse_tlv(input)?;
    if tag.class != 0 || tag.number != num {
        anyhow::bail!(
            "unexpected DER tag: expected universal {num}, got {:?}",
            tag
        );
    }
    Ok((whole, val, rest))
}

fn parse_der_int_i64(bytes: &[u8]) -> anyhow::Result<i64> {
    if bytes.is_empty() || bytes.len() > 8 {
        anyhow::bail!("DER integer out of i64 range");
    }
    let mut val = if (bytes[0] & 0x80) != 0 { -1i64 } else { 0i64 };
    for &b in bytes {
        val = (val << 8) | i64::from(b);
    }
    Ok(val)
}

fn parse_x509_time(node: &[u8]) -> anyhow::Result<(DateTime<Utc>, &[u8])> {
    let (tag, _whole, val, rest) = parse_tlv(node)?;
    let s = std::str::from_utf8(val)?;
    let dt = match (tag.class, tag.number) {
        (0, 23) => {
            // UTCTime: YYMMDDHHMMSSZ
            let s = s
                .strip_suffix('Z')
                .ok_or_else(|| anyhow::anyhow!("UTCTime missing Z"))?;
            if s.len() != 12 {
                anyhow::bail!("invalid UTCTime length");
            }
            let yy: i32 = s[..2].parse()?;
            let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
            let full = format!("{year:04}{}", &s[2..]);
            let naive = NaiveDateTime::parse_from_str(&full, "%Y%m%d%H%M%S")?;
            DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
        }
        (0, 24) => {
            // GeneralizedTime: YYYYMMDDHHMMSSZ
            let s = s
                .strip_suffix('Z')
                .ok_or_else(|| anyhow::anyhow!("GeneralizedTime missing Z"))?;
            let naive = NaiveDateTime::parse_from_str(s, "%Y%m%d%H%M%S")?;
            DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
        }
        _ => anyhow::bail!("expected X.509 time tag"),
    };
    Ok((dt, rest))
}

struct ParsedCert<'a> {
    tbs_der: &'a [u8],
    serial_hex: String,
    sig_alg_oid: &'a [u8],
    sig_bits: &'a [u8],
    issuer_der: &'a [u8],
    subject_der: &'a [u8],
    not_before: DateTime<Utc>,
    not_after: DateTime<Utc>,
    spki_der: &'a [u8],
    spki_alg_oid: &'a [u8],
    spki_param_oid: Option<&'a [u8]>,
    spki_pubkey: &'a [u8],
    is_ca: bool,
    attestation_ext: Option<&'a [u8]>,
}

fn parse_alg_id(input: &[u8]) -> anyhow::Result<(&[u8], Option<&[u8]>, &[u8])> {
    let (_w, seq, rest) = expect_universal(input, 16)?;
    let (_w2, oid, after_oid) = expect_universal(seq, 6)?;
    let param_oid = if !after_oid.is_empty() {
        let (ptag, _pw, pval, _prest) = parse_tlv(after_oid)?;
        if ptag.class == 0 && ptag.number == 6 {
            Some(pval)
        } else {
            None
        }
    } else {
        None
    };
    Ok((oid, param_oid, rest))
}

fn parse_cert(der: &[u8]) -> anyhow::Result<ParsedCert<'_>> {
    let (_w, cert_body, trailing) = expect_universal(der, 16)?;
    if !trailing.is_empty() {
        anyhow::bail!("trailing bytes after X.509 Certificate");
    }
    let (tbs_der, tbs_body, after_tbs) = expect_universal(cert_body, 16)?;
    let (sig_alg_oid, _, after_sig_alg) = parse_alg_id(after_tbs)?;
    let (_w_sig, sig_bitstring, after_sig) = expect_universal(after_sig_alg, 3)?;
    if !after_sig.is_empty() || sig_bitstring.is_empty() || sig_bitstring[0] != 0 {
        anyhow::bail!("invalid Certificate signatureValue BIT STRING");
    }
    let sig_bits = &sig_bitstring[1..];

    // Inside tbsCertificate: optional [0] version
    let mut cur = tbs_body;
    let (first_tag, _w0, _v0, after0) = parse_tlv(cur)?;
    if first_tag.class == 2 && first_tag.number == 0 {
        cur = after0;
    }
    let (_ws, serial_bytes, cur) = expect_universal(cur, 2)?;
    let (_tbs_sig_oid, _, cur) = parse_alg_id(cur)?;
    let (issuer_der, _issuer_val, cur) = expect_universal(cur, 16)?;
    let (_wv, validity_body, cur) = expect_universal(cur, 16)?;
    let (not_before, vrest) = parse_x509_time(validity_body)?;
    let (not_after, vrest2) = parse_x509_time(vrest)?;
    if !vrest2.is_empty() {
        anyhow::bail!("unexpected bytes in X.509 Validity");
    }
    let (subject_der, _subj_val, cur) = expect_universal(cur, 16)?;
    let (spki_der, spki_body, mut cur) = expect_universal(cur, 16)?;

    let (spki_alg_oid, spki_param_oid, spki_rest) = parse_alg_id(spki_body)?;
    let (_w_pk, pk_bits, pk_tail) = expect_universal(spki_rest, 3)?;
    if !pk_tail.is_empty() || pk_bits.is_empty() || pk_bits[0] != 0 {
        anyhow::bail!("invalid SPKI BIT STRING");
    }
    let spki_pubkey = &pk_bits[1..];

    let mut is_ca = false;
    let mut attestation_ext = None;

    while !cur.is_empty() {
        let (t, _w, val, next) = parse_tlv(cur)?;
        cur = next;
        // [3] EXPLICIT Extensions
        if t.class == 2 && t.number == 3 {
            let (_we, mut exts_body, ext_tail) = expect_universal(val, 16)?;
            if !ext_tail.is_empty() {
                anyhow::bail!("trailing bytes in Extensions");
            }
            while !exts_body.is_empty() {
                let (_wext, ext_seq, erest) = expect_universal(exts_body, 16)?;
                exts_body = erest;
                let (_woid, oid, mut ecur) = expect_universal(ext_seq, 6)?;
                if let Ok((bt, _bw, _bv, brest)) = parse_tlv(ecur) {
                    if bt.class == 0 && bt.number == 1 {
                        ecur = brest;
                    }
                }
                let (_wval, extn_value, etail) = expect_universal(ecur, 4)?;
                if !etail.is_empty() {
                    anyhow::bail!("trailing bytes in X.509 Extension");
                }
                if oid == OID_BASIC_CONSTRAINTS {
                    let (_wb, bc_body, _) = expect_universal(extn_value, 16)?;
                    if !bc_body.is_empty() {
                        if let Ok((btag, _, bval, _)) = parse_tlv(bc_body) {
                            if btag.class == 0
                                && btag.number == 1
                                && !bval.is_empty()
                                && bval[0] != 0
                            {
                                is_ca = true;
                            }
                        }
                    }
                } else if oid == OID_ANDROID_ATTESTATION {
                    attestation_ext = Some(extn_value);
                }
            }
        }
    }

    Ok(ParsedCert {
        tbs_der,
        serial_hex: normalize_serial_bytes(serial_bytes),
        sig_alg_oid,
        sig_bits,
        issuer_der,
        subject_der,
        not_before,
        not_after,
        spki_der,
        spki_alg_oid,
        spki_param_oid,
        spki_pubkey,
        is_ca,
        attestation_ext,
    })
}

fn verify_cert_sig(child: &ParsedCert<'_>, issuer: &ParsedCert<'_>) -> anyhow::Result<()> {
    let alg: &'static dyn signature::VerificationAlgorithm = match (
        child.sig_alg_oid,
        issuer.spki_alg_oid,
        issuer.spki_param_oid,
    ) {
        (OID_SHA256_WITH_RSA, OID_RSA_ENCRYPTION, _) => &signature::RSA_PKCS1_2048_8192_SHA256,
        (OID_SHA384_WITH_RSA, OID_RSA_ENCRYPTION, _) => &signature::RSA_PKCS1_2048_8192_SHA384,
        (OID_SHA512_WITH_RSA, OID_RSA_ENCRYPTION, _) => &signature::RSA_PKCS1_2048_8192_SHA512,
        (OID_ECDSA_WITH_SHA256, OID_EC_PUBLIC_KEY, Some(OID_PRIME256V1)) => {
            &signature::ECDSA_P256_SHA256_ASN1
        }
        (OID_ECDSA_WITH_SHA384, OID_EC_PUBLIC_KEY, Some(OID_SECP384R1)) => {
            &signature::ECDSA_P384_SHA384_ASN1
        }
        _ => anyhow::bail!("unsupported certificate signature/public-key algorithm"),
    };
    signature::UnparsedPublicKey::new(alg, issuer.spki_pubkey)
        .verify(child.tbs_der, child.sig_bits)
        .map_err(|_| anyhow::anyhow!("certificate signature verification failed"))?;
    Ok(())
}

fn parse_auth_list(seq_bytes: &[u8]) -> anyhow::Result<AuthListSummary> {
    let mut cur = seq_bytes;
    let mut out = AuthListSummary::default();
    while !cur.is_empty() {
        let (tag, _w, val, next) = parse_tlv(cur)?;
        cur = next;
        if tag.class != 2 {
            continue;
        }
        match tag.number {
            1 => {
                // purpose SET OF INTEGER
                let (_ws, mut set_body, _) = expect_universal(val, 17)?;
                while !set_body.is_empty() {
                    let (_wi, ib, rest) = expect_universal(set_body, 2)?;
                    set_body = rest;
                    if parse_der_int_i64(ib)? == 2 {
                        out.has_sign_purpose = true;
                    }
                }
            }
            2 => {
                let (_wi, ib, _) = expect_universal(val, 2)?;
                out.algorithm = Some(parse_der_int_i64(ib)?);
            }
            3 => {
                let (_wi, ib, _) = expect_universal(val, 2)?;
                out.key_size = Some(parse_der_int_i64(ib)?);
            }
            503 => {
                let (_wi, ib, _) = expect_universal(val, 2)?;
                out.user_auth_type = Some(parse_der_int_i64(ib)?);
            }
            504 => {
                out.no_auth_required = true;
            }
            505 => {
                let (_wi, ib, _) = expect_universal(val, 2)?;
                out.auth_timeout = Some(parse_der_int_i64(ib)?);
            }
            702 => {
                let (_wi, ib, _) = expect_universal(val, 2)?;
                out.origin = Some(parse_der_int_i64(ib)?);
            }
            704 => {
                out.root_of_trust = Some(parse_root_of_trust(val)?);
            }
            709 => {
                let (_wo, oct, _) = expect_universal(val, 4)?;
                out.app_id = Some(parse_attestation_app_id(oct)?);
            }
            _ => {}
        }
    }
    Ok(out)
}

fn parse_root_of_trust(der: &[u8]) -> anyhow::Result<RootOfTrust> {
    let (_w, body, rest) = expect_universal(der, 16)?;
    if !rest.is_empty() {
        anyhow::bail!("trailing bytes in RootOfTrust");
    }
    let (_wk, _boot_key, cur) = expect_universal(body, 4)?;
    let (_wl, locked_b, cur) = expect_universal(cur, 1)?;
    if locked_b.len() != 1 {
        anyhow::bail!("invalid RootOfTrust deviceLocked BOOLEAN");
    }
    let (_ws, vbs_b, cur) = expect_universal(cur, 10)?;
    let verified_boot_state = parse_der_int_i64(vbs_b)?;
    if !cur.is_empty() {
        let (_wh, _boot_hash, tail) = expect_universal(cur, 4)?;
        if !tail.is_empty() {
            anyhow::bail!("trailing fields in RootOfTrust");
        }
    }
    Ok(RootOfTrust {
        device_locked: locked_b[0] != 0,
        verified_boot_state,
    })
}

#[derive(Default)]
struct RootOfTrust {
    device_locked: bool,
    verified_boot_state: i64,
}

#[derive(Default)]
struct AttestationAppId {
    packages: Vec<String>,
    signature_digests: Vec<Vec<u8>>,
}

#[derive(Default)]
struct AuthListSummary {
    has_sign_purpose: bool,
    algorithm: Option<i64>,
    key_size: Option<i64>,
    user_auth_type: Option<i64>,
    no_auth_required: bool,
    auth_timeout: Option<i64>,
    origin: Option<i64>,
    root_of_trust: Option<RootOfTrust>,
    app_id: Option<AttestationAppId>,
}

fn parse_attestation_app_id(der: &[u8]) -> anyhow::Result<AttestationAppId> {
    let (_w, body, rest) = expect_universal(der, 16)?;
    if !rest.is_empty() {
        anyhow::bail!("trailing bytes in AttestationApplicationId");
    }
    let (_wp, mut pkgs_set, cur) = expect_universal(body, 17)?;
    let (_ws, mut sigs_set, tail) = expect_universal(cur, 17)?;
    if !tail.is_empty() {
        anyhow::bail!("trailing fields in AttestationApplicationId");
    }
    let mut out = AttestationAppId::default();
    while !pkgs_set.is_empty() {
        let (_w, pkg_seq, next) = expect_universal(pkgs_set, 16)?;
        pkgs_set = next;
        let (_wn, pkg_oct, _) = expect_universal(pkg_seq, 4)?;
        if let Ok(pkg) = std::str::from_utf8(pkg_oct) {
            out.packages.push(pkg.to_string());
        }
    }
    while !sigs_set.is_empty() {
        let (_w, dig_oct, next) = expect_universal(sigs_set, 4)?;
        sigs_set = next;
        out.signature_digests.push(dig_oct.to_vec());
    }
    Ok(out)
}

fn decode_fingerprints(fps: &[String]) -> anyhow::Result<Vec<Vec<u8>>> {
    let mut out = Vec::with_capacity(fps.len());
    for fp in fps {
        let clean: String = fp.chars().filter(|c| *c != ':').collect();
        let raw = hex::decode(clean)?;
        if raw.len() != 32 {
            anyhow::bail!("Android SHA-256 fingerprint must be 32 bytes");
        }
        out.push(raw);
    }
    Ok(out)
}

/// Configured trusted Android attestation root SPKI SHA-256 hex strings.
pub fn trusted_root_spki_hashes() -> Vec<String> {
    if let Ok(raw) = std::env::var("ANDROID_ATTESTATION_ROOT_SPKI_SHA256") {
        let custom: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .collect();
        if !custom.is_empty() {
            return custom;
        }
    }
    GOOGLE_ATTESTATION_ROOT_SPKI_SHA256
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

static REVOKED_SERIALS: std::sync::OnceLock<std::sync::RwLock<std::collections::HashSet<String>>> =
    std::sync::OnceLock::new();

fn revoked_cache() -> &'static std::sync::RwLock<std::collections::HashSet<String>> {
    REVOKED_SERIALS.get_or_init(|| std::sync::RwLock::new(std::collections::HashSet::new()))
}

fn normalize_serial_str(raw: &str) -> String {
    let stripped: String = raw
        .trim()
        .strip_prefix("0x")
        .or_else(|| raw.trim().strip_prefix("0X"))
        .unwrap_or(raw.trim())
        .chars()
        .filter(|c| *c != ':' && !c.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    let trimmed = stripped.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_serial_bytes(bytes: &[u8]) -> String {
    normalize_serial_str(&hex::encode(bytes))
}

pub fn update_revoked_serials(serials: std::collections::HashSet<String>) {
    let normalized: std::collections::HashSet<String> = serials
        .into_iter()
        .map(|s| normalize_serial_str(&s))
        .collect();
    if let Ok(mut guard) = revoked_cache().write() {
        *guard = normalized;
    }
}

pub fn is_serial_revoked(serial_hex: &str) -> bool {
    let norm = normalize_serial_str(serial_hex);
    revoked_cache()
        .read()
        .map(|g| g.contains(&norm))
        .unwrap_or(false)
}

pub async fn refresh_google_attestation_revocation_list(
    client: &reqwest::Client,
) -> anyhow::Result<usize> {
    let resp = client
        .get("https://android.googleapis.com/attestation/status")
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("attestation CRL fetch returned HTTP {}", resp.status());
    }
    let body = crate::util::read_response_bytes_bounded(resp, 2 * 1024 * 1024).await?;
    let val: serde_json::Value = serde_json::from_slice(&body)?;
    let entries = val
        .get("entries")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("attestation CRL JSON missing entries object"))?;
    let next: std::collections::HashSet<String> =
        entries.keys().map(|k| normalize_serial_str(k)).collect();
    let count = next.len();
    update_revoked_serials(next);
    Ok(count)
}

pub fn require_attestation_enabled_for(dev_mode: bool, android_certs: &[String]) -> bool {
    match std::env::var("REQUIRE_BIOMETRIC_ATTESTATION") {
        Ok(v) if !v.trim().is_empty() => {
            let l = v.trim().to_ascii_lowercase();
            l == "true" || l == "1"
        }
        _ => !dev_mode && !android_certs.is_empty(),
    }
}

pub fn require_attestation_enabled() -> bool {
    require_attestation_enabled_for(false, &[])
}

pub fn verify_android_key_attestation(
    chain_der: &[Vec<u8>],
    expected_pubkey_65: &[u8],
    expected_challenge: &str,
    expected_cert_sha256_fps: &[String],
    allowed_root_spki_sha256: &[String],
    now: DateTime<Utc>,
) -> anyhow::Result<()> {
    let snapshot = revoked_cache()
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    verify_android_key_attestation_with_revoked(
        chain_der,
        expected_pubkey_65,
        expected_challenge,
        expected_cert_sha256_fps,
        allowed_root_spki_sha256,
        now,
        &snapshot,
    )
}

pub fn verify_android_key_attestation_with_revoked(
    chain_der: &[Vec<u8>],
    expected_pubkey_65: &[u8],
    expected_challenge: &str,
    expected_cert_sha256_fps: &[String],
    allowed_root_spki_sha256: &[String],
    now: DateTime<Utc>,
    revoked_serials: &std::collections::HashSet<String>,
) -> anyhow::Result<()> {
    if chain_der.len() < 2 || chain_der.len() > 8 {
        anyhow::bail!("attestation certificate chain must contain 2..=8 certificates");
    }
    let mut certs = Vec::with_capacity(chain_der.len());
    for der in chain_der {
        let c = parse_cert(der)?;
        if now < c.not_before || now > c.not_after {
            anyhow::bail!("attestation certificate expired or not yet valid");
        }
        if revoked_serials.contains(&c.serial_hex) {
            anyhow::bail!("attestation certificate serial is revoked");
        }
        certs.push(c);
    }

    // 1. Leaf key matches submitted 65-byte EC P-256 public key.
    let leaf = &certs[0];
    if leaf.spki_alg_oid != OID_EC_PUBLIC_KEY
        || leaf.spki_param_oid != Some(OID_PRIME256V1)
        || leaf.spki_pubkey != expected_pubkey_65
    {
        anyhow::bail!("attestation leaf certificate public key mismatch");
    }

    // 2. Verify signatures & BasicConstraints up to a pinned root CA.
    for i in 0..certs.len() - 1 {
        if certs[i].issuer_der != certs[i + 1].subject_der {
            anyhow::bail!("attestation certificate chain issuer mismatch");
        }
        if !certs[i + 1].is_ca {
            anyhow::bail!(
                "attestation intermediate/root certificate lacks BasicConstraints CA=TRUE"
            );
        }
        verify_cert_sig(&certs[i], &certs[i + 1])?;
    }
    let root = certs.last().expect("chain non-empty");
    if root.subject_der != root.issuer_der {
        anyhow::bail!("attestation root certificate is not self-issued");
    }
    verify_cert_sig(root, root)?;
    let root_spki_hash = hex::encode(Sha256::digest(root.spki_der));
    if !allowed_root_spki_sha256
        .iter()
        .any(|h| h.eq_ignore_ascii_case(&root_spki_hash))
    {
        anyhow::bail!("attestation root CA SPKI is not trusted");
    }

    // 3. Parse Android Key Attestation extension on leaf certificate.
    let ext = leaf
        .attestation_ext
        .ok_or_else(|| anyhow::anyhow!("missing Android Key Attestation extension"))?;
    let (_w, desc_body, tail) = expect_universal(ext, 16)?;
    if !tail.is_empty() {
        anyhow::bail!("trailing bytes in KeyDescription");
    }
    let (_w1, ver_b, cur) = expect_universal(desc_body, 2)?;
    let (_w2, att_sec_b, cur) = expect_universal(cur, 10)?; // ENUMERATED
    let (_w3, _km_ver_b, cur) = expect_universal(cur, 2)?;
    let (_w4, km_sec_b, cur) = expect_universal(cur, 10)?; // ENUMERATED
    let (_w5, challenge_b, cur) = expect_universal(cur, 4)?;
    let (_w6, _unique_id_b, cur) = expect_universal(cur, 4)?;
    let (_w7, sw_seq, cur) = expect_universal(cur, 16)?;
    let (_w8, tee_seq, desc_tail) = expect_universal(cur, 16)?;
    if !desc_tail.is_empty() {
        anyhow::bail!("unexpected fields in KeyDescription");
    }

    if parse_der_int_i64(ver_b)? < 1 {
        anyhow::bail!("invalid attestationVersion");
    }
    // 1 = TrustedEnvironment, 2 = StrongBox (0 = Software -> reject)
    if parse_der_int_i64(att_sec_b)? < 1 || parse_der_int_i64(km_sec_b)? < 1 {
        anyhow::bail!("biometric key must be hardware-backed (TEE or StrongBox)");
    }
    if challenge_b != expected_challenge.as_bytes() {
        anyhow::bail!("attestationChallenge mismatch");
    }

    let sw = parse_auth_list(sw_seq)?;
    let tee = parse_auth_list(tee_seq)?;
    if !tee.has_sign_purpose
        || tee.algorithm != Some(3)
        || tee.key_size != Some(256)
        || tee.no_auth_required
        || tee.user_auth_type != Some(2)
        || tee.origin != Some(0)
        || tee.auth_timeout.is_some_and(|t| t != 0)
    {
        anyhow::bail!(
            "TEE authorization list must enforce generated, per-use biometric P-256 signing"
        );
    }
    let rot = tee
        .root_of_trust
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("TEE authorization list missing rootOfTrust"))?;
    if !rot.device_locked || rot.verified_boot_state != 0 {
        anyhow::bail!("device must have locked bootloader and Verified boot state");
    }

    // Check APK package name & signing certificate SHA-256 when configured.
    let app_id = sw
        .app_id
        .or(tee.app_id)
        .ok_or_else(|| anyhow::anyhow!("missing attestationApplicationId"))?;
    if !app_id
        .packages
        .iter()
        .any(|p| p == EXPECTED_ANDROID_PACKAGE)
    {
        anyhow::bail!("attestationApplicationId package mismatch");
    }
    if !expected_cert_sha256_fps.is_empty() {
        let wanted = decode_fingerprints(expected_cert_sha256_fps)?;
        let matches_sig = app_id
            .signature_digests
            .iter()
            .any(|sig| wanted.iter().any(|w| w == sig));
        if !matches_sig {
            anyhow::bail!("attestationApplicationId signing certificate mismatch");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_root_hashes_are_valid_hex_sha256() {
        assert_eq!(GOOGLE_ATTESTATION_ROOT_SPKI_SHA256.len(), 2);
        for h in GOOGLE_ATTESTATION_ROOT_SPKI_SHA256 {
            assert_eq!(h.len(), 64);
            assert!(hex::decode(h).unwrap().len() == 32);
        }
    }

    #[test]
    fn high_tag_and_app_id_round_trip() {
        // Test parse_tlv on context-specific [503] and [709] high tags.
        let mut buf = vec![0xbf, 0x83, 0x77, 0x03, 0x02, 0x01, 0x02]; // [503] INTEGER 2
        let (tag, _w, val, rest) = parse_tlv(&buf).unwrap();
        assert_eq!(tag.class, 2);
        assert_eq!(tag.number, 503);
        assert!(rest.is_empty());
        let (_wi, ib, _) = expect_universal(val, 2).unwrap();
        assert_eq!(parse_der_int_i64(ib).unwrap(), 2);

        buf = vec![0x9f, 0x85, 0x45, 0x00]; // [709] primitive len 0
        let (tag2, _, _, _) = parse_tlv(&buf).unwrap();
        assert_eq!(tag2.number, 709);
    }

    fn push_len(out: &mut Vec<u8>, len: usize) {
        if len < 0x80 {
            out.push(len as u8);
        } else {
            let bytes = len.to_be_bytes();
            let start = bytes.iter().position(|b| *b != 0).unwrap();
            let slice = &bytes[start..];
            out.push(0x80 | (slice.len() as u8));
            out.extend_from_slice(slice);
        }
    }

    fn der_tlv(tag: u8, val: &[u8]) -> Vec<u8> {
        let mut out = vec![tag];
        push_len(&mut out, val.len());
        out.extend_from_slice(val);
        out
    }

    fn der_seq(parts: &[&[u8]]) -> Vec<u8> {
        let mut body = Vec::new();
        for p in parts {
            body.extend_from_slice(p);
        }
        der_tlv(0x30, &body)
    }

    fn der_set(parts: &[&[u8]]) -> Vec<u8> {
        let mut body = Vec::new();
        for p in parts {
            body.extend_from_slice(p);
        }
        der_tlv(0x31, &body)
    }

    fn der_int(v: i64) -> Vec<u8> {
        let bytes = v.to_be_bytes();
        let mut idx = 0;
        while idx < 7 {
            if (bytes[idx] == 0x00 && (bytes[idx + 1] & 0x80) == 0)
                || (bytes[idx] == 0xff && (bytes[idx + 1] & 0x80) != 0)
            {
                idx += 1;
            } else {
                break;
            }
        }
        der_tlv(0x02, &bytes[idx..])
    }

    fn der_enum(v: i64) -> Vec<u8> {
        let mut node = der_int(v);
        node[0] = 0x0a;
        node
    }

    fn der_bitstring(bytes: &[u8]) -> Vec<u8> {
        let mut val = Vec::with_capacity(1 + bytes.len());
        val.push(0x00);
        val.extend_from_slice(bytes);
        der_tlv(0x03, &val)
    }

    fn der_ctx_explicit(num: u32, inner: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if num < 31 {
            out.push(0xA0 | (num as u8));
        } else {
            out.push(0xBF);
            let mut parts = Vec::new();
            let mut v = num;
            parts.push((v & 0x7f) as u8);
            v >>= 7;
            while v > 0 {
                parts.push(((v & 0x7f) as u8) | 0x80);
                v >>= 7;
            }
            parts.reverse();
            out.extend_from_slice(&parts);
        }
        push_len(&mut out, inner.len());
        out.extend_from_slice(inner);
        out
    }

    struct AttestCaseOpts {
        user_auth_type: i64,
        auth_timeout: Option<i64>,
        origin: Option<i64>,
        include_root_of_trust: bool,
        device_locked: bool,
        verified_boot_state: i64,
    }

    impl Default for AttestCaseOpts {
        fn default() -> Self {
            Self {
                user_auth_type: 2,
                auth_timeout: Some(0),
                origin: Some(0),
                include_root_of_trust: true,
                device_locked: true,
                verified_boot_state: 0,
            }
        }
    }

    fn build_synthetic_chain(
        opts: AttestCaseOpts,
        challenge: &str,
        cert_sha256: &[u8; 32],
    ) -> (Vec<Vec<u8>>, Vec<u8>, String) {
        use ring::rand::SystemRandom;
        use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_ASN1_SIGNING};

        let rng = SystemRandom::new();
        let root_pkcs8 =
            EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
        let root_kp =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, root_pkcs8.as_ref(), &rng)
                .unwrap();
        let leaf_pkcs8 =
            EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng).unwrap();
        let leaf_kp =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, leaf_pkcs8.as_ref(), &rng)
                .unwrap();

        let ec_alg = der_seq(&[
            &der_tlv(0x06, OID_EC_PUBLIC_KEY),
            &der_tlv(0x06, OID_PRIME256V1),
        ]);
        let sig_alg = der_seq(&[&der_tlv(0x06, OID_ECDSA_WITH_SHA256)]);
        let root_spki = der_seq(&[&ec_alg, &der_bitstring(root_kp.public_key().as_ref())]);
        let leaf_spki = der_seq(&[&ec_alg, &der_bitstring(leaf_kp.public_key().as_ref())]);
        let root_spki_hash = hex::encode(Sha256::digest(&root_spki));

        let validity = der_seq(&[
            &der_tlv(0x17, b"250101000000Z"),
            &der_tlv(0x17, b"350101000000Z"),
        ]);
        let rdn = |cn: &[u8]| {
            let atv = der_seq(&[&der_tlv(0x06, &[0x55, 0x04, 0x03]), &der_tlv(0x0c, cn)]);
            der_seq(&[&der_set(&[&atv])])
        };
        let root_name = rdn(b"Test Root");
        let leaf_name = rdn(b"Test Leaf");

        let bc_true = der_seq(&[
            &der_tlv(0x06, OID_BASIC_CONSTRAINTS),
            &der_tlv(0x04, &der_seq(&[&[0x01, 0x01, 0xff]])),
        ]);
        let root_tbs = der_seq(&[
            &der_ctx_explicit(0, &der_int(2)),
            &der_int(1),
            &sig_alg,
            &root_name,
            &validity,
            &root_name,
            &root_spki,
            &der_ctx_explicit(3, &der_seq(&[&bc_true])),
        ]);
        let root_sig = root_kp.sign(&rng, &root_tbs).unwrap();
        let root_cert = der_seq(&[&root_tbs, &sig_alg, &der_bitstring(root_sig.as_ref())]);

        // Android attestation extension
        let app_id_der = der_seq(&[
            &der_set(&[&der_seq(&[
                &der_tlv(0x04, EXPECTED_ANDROID_PACKAGE.as_bytes()),
                &der_int(1),
            ])]),
            &der_set(&[&der_tlv(0x04, cert_sha256)]),
        ]);
        let sw_list = der_seq(&[&der_ctx_explicit(709, &der_tlv(0x04, &app_id_der))]);

        let mut tee_fields = vec![
            der_ctx_explicit(1, &der_set(&[&der_int(2)])),
            der_ctx_explicit(2, &der_int(3)),
            der_ctx_explicit(3, &der_int(256)),
            der_ctx_explicit(503, &der_int(opts.user_auth_type)),
        ];
        if let Some(t) = opts.auth_timeout {
            tee_fields.push(der_ctx_explicit(505, &der_int(t)));
        }
        if let Some(o) = opts.origin {
            tee_fields.push(der_ctx_explicit(702, &der_int(o)));
        }
        if opts.include_root_of_trust {
            let boot_key = [0x11u8; 32];
            let locked = [0x01, 0x01, if opts.device_locked { 0xff } else { 0x00 }];
            let vbs = der_enum(opts.verified_boot_state);
            let rot = der_seq(&[&der_tlv(0x04, &boot_key), &locked, &vbs]);
            tee_fields.push(der_ctx_explicit(704, &rot));
        }
        let tee_refs: Vec<&[u8]> = tee_fields.iter().map(Vec::as_slice).collect();
        let tee_list = der_seq(&tee_refs);

        let key_desc = der_seq(&[
            &der_int(3),
            &der_enum(1),
            &der_int(4),
            &der_enum(1),
            &der_tlv(0x04, challenge.as_bytes()),
            &der_tlv(0x04, b""),
            &sw_list,
            &tee_list,
        ]);
        let att_ext = der_seq(&[
            &der_tlv(0x06, OID_ANDROID_ATTESTATION),
            &der_tlv(0x04, &key_desc),
        ]);
        let bc_false = der_seq(&[
            &der_tlv(0x06, OID_BASIC_CONSTRAINTS),
            &der_tlv(0x04, &der_seq(&[])),
        ]);
        let leaf_tbs = der_seq(&[
            &der_ctx_explicit(0, &der_int(2)),
            &der_int(2),
            &sig_alg,
            &root_name,
            &validity,
            &leaf_name,
            &leaf_spki,
            &der_ctx_explicit(3, &der_seq(&[&bc_false, &att_ext])),
        ]);
        let leaf_sig = root_kp.sign(&rng, &leaf_tbs).unwrap();
        let leaf_cert = der_seq(&[&leaf_tbs, &sig_alg, &der_bitstring(leaf_sig.as_ref())]);

        (
            vec![leaf_cert, root_cert],
            leaf_kp.public_key().as_ref().to_vec(),
            root_spki_hash,
        )
    }

    #[test]
    fn android_attestation_enforces_boot_integrity_origin_and_per_use_biometrics() {
        let cert_fp_bytes = [0xAAu8; 32];
        let cert_fp_hex = hex::encode(cert_fp_bytes);
        let now = DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let (chain, pubkey, root_hash) =
            build_synthetic_chain(AttestCaseOpts::default(), "chal-ok", &cert_fp_bytes);
        verify_android_key_attestation(
            &chain,
            &pubkey,
            "chal-ok",
            std::slice::from_ref(&cert_fp_hex),
            std::slice::from_ref(&root_hash),
            now,
        )
        .expect("locked Verified Boot, GENERATED, biometric-only key must pass");

        let bad_cases = [
            (
                "unlocked bootloader",
                AttestCaseOpts {
                    device_locked: false,
                    ..Default::default()
                },
            ),
            (
                "self-signed boot state",
                AttestCaseOpts {
                    verified_boot_state: 1,
                    ..Default::default()
                },
            ),
            (
                "missing rootOfTrust",
                AttestCaseOpts {
                    include_root_of_trust: false,
                    ..Default::default()
                },
            ),
            (
                "imported origin",
                AttestCaseOpts {
                    origin: Some(2),
                    ..Default::default()
                },
            ),
            (
                "missing origin",
                AttestCaseOpts {
                    origin: None,
                    ..Default::default()
                },
            ),
            (
                "non-zero authTimeout",
                AttestCaseOpts {
                    auth_timeout: Some(60),
                    ..Default::default()
                },
            ),
            (
                "PIN/password allowed in userAuthType=3",
                AttestCaseOpts {
                    user_auth_type: 3,
                    ..Default::default()
                },
            ),
        ];

        for (label, opts) in bad_cases {
            let (bad_chain, bad_pk, bad_root) =
                build_synthetic_chain(opts, "chal-bad", &cert_fp_bytes);
            assert!(
                verify_android_key_attestation(
                    &bad_chain,
                    &bad_pk,
                    "chal-bad",
                    std::slice::from_ref(&cert_fp_hex),
                    &[bad_root],
                    now,
                )
                .is_err(),
                "{label} must be rejected"
            );
        }
    }
}
