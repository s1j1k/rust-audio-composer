//! Optional HTTPS for the remote keyboard.
//!
//! iOS only exposes the accelerometer (`DeviceMotionEvent`) to pages served from a
//! *secure context*. We use it to measure how hard a key was tapped — the same trick
//! mobile DAWs use for velocity on glass. On a LAN there is no public certificate, so we
//! create a tiny local certificate authority once (kept in the app's data folder) and issue
//! a fresh server certificate for the current LAN IPs on every start.
//!
//! Install the CA on the iPhone once (Settings → Profile Downloaded → Install, then
//! Settings → General → About → Certificate Trust Settings → enable full trust) and the
//! HTTPS page loads without warnings. Nothing ever leaves the local network.

use crate::audio::samples::home_dir;
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose,
    IsCa, KeyPair, KeyUsagePurpose, SanType,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TlsMaterial {
    pub server_config: Arc<rustls::ServerConfig>,
    /// DER-encoded CA certificate, served at `/ca.crt` for installing on the phone.
    pub ca_der: Vec<u8>,
}

fn dir() -> PathBuf {
    home_dir().join("rust-audio-composer").join("remote")
}

fn ca_params() -> CertificateParams {
    let mut p = CertificateParams::default();
    p.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Rust Audio Composer Local CA");
    dn.push(DnType::OrganizationName, "Rust Audio Composer (this computer only)");
    p.distinguished_name = dn;
    p.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    p.not_before = rcgen::date_time_ymd(2025, 1, 1);
    p.not_after = rcgen::date_time_ymd(2035, 1, 1);
    p
}

/// Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i32, u8, u8) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    ((y + if m <= 2 { 1 } else { 0 }) as i32, m, d)
}

fn ymd_from_now(days: i64) -> (i32, u8, u8) {
    let today = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 / 86_400)
        .unwrap_or(20_000);
    civil_from_days(today + days)
}

fn load_or_create_ca() -> Result<(KeyPair, Vec<u8>), String> {
    let dir = dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let key_path = dir.join("ca.key.pem");
    let der_path = dir.join("ca.crt");
    if let (Ok(pem), Ok(der)) = (fs::read_to_string(&key_path), fs::read(&der_path)) {
        if let Ok(key) = KeyPair::from_pem(&pem) {
            return Ok((key, der));
        }
    }
    let key = KeyPair::generate().map_err(|e| e.to_string())?;
    let cert = ca_params().self_signed(&key).map_err(|e| e.to_string())?;
    let der = cert.der().to_vec();
    fs::write(&key_path, key.serialize_pem()).map_err(|e| e.to_string())?;
    fs::write(&der_path, &der).map_err(|e| e.to_string())?;
    Ok((key, der))
}

pub fn load_or_create(ips: &[IpAddr], hostname: Option<&str>) -> Result<TlsMaterial, String> {
    let (ca_key, ca_der) = load_or_create_ca()?;
    // Re-create the issuer in memory from the same key + name; certificates it signs
    // verify against the CA the phone installed earlier.
    let ca_cert = ca_params().self_signed(&ca_key).map_err(|e| e.to_string())?;

    let mut dns = vec!["localhost".to_string()];
    if let Some(h) = hostname {
        dns.push(format!("{h}.local"));
    }
    let mut params = CertificateParams::new(dns).map_err(|e| e.to_string())?;
    for ip in ips {
        params.subject_alt_names.push(SanType::IpAddress(*ip));
    }
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "Rust Audio Composer remote keyboard");
    params.distinguished_name = dn;
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    // Apple rejects server certificates valid for more than 825 days.
    let (y, m, d) = ymd_from_now(-2);
    params.not_before = rcgen::date_time_ymd(y, m, d);
    let (y, m, d) = ymd_from_now(800);
    params.not_after = rcgen::date_time_ymd(y, m, d);

    let leaf_key = KeyPair::generate().map_err(|e| e.to_string())?;
    let leaf = params
        .signed_by(&leaf_key, &ca_cert, &ca_key)
        .map_err(|e| e.to_string())?;

    let chain = vec![
        CertificateDer::from(leaf.der().to_vec()),
        CertificateDer::from(ca_der.clone()),
    ];
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(|e| e.to_string())?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(TlsMaterial {
        server_config: Arc::new(config),
        ca_der,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(20_403), (2025, 11, 11));
    }
}
