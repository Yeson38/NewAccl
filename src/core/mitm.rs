use rcgen::{CertificateParams, KeyPair, DnType};
use std::fs;
use anyhow::Result;

pub const CA_CERT_PATH: &str = "ca.crt";
pub const CA_KEY_PATH: &str = "ca.key";

pub fn init_ca() -> Result<()> {
    if fs::metadata(CA_CERT_PATH).is_ok() && fs::metadata(CA_KEY_PATH).is_ok() {
        return Ok(());
    }

    let mut params = CertificateParams::default();
    params.distinguished_name.push(DnType::CommonName, "NewAccl Root CA");
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    fs::write(CA_CERT_PATH, cert.pem())?;
    fs::write(CA_KEY_PATH, key_pair.serialize_pem())?;
    
    tracing::info!("Generated local CA at {} and {}", CA_CERT_PATH, CA_KEY_PATH);
    Ok(())
}