use std::{fs, path::PathBuf};

use color_eyre::eyre::{bail, Context, Result};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CertificateSigningRequest, DistinguishedName,
    DnType, DnValue, IsCa, KeyPair, PKCS_ECDSA_P256_SHA256,
};
use rustls::client::{ServerCertVerified, ServerCertVerifier};
use rustls::server::ParsedCertificate;
use rustls::RootCertStore;
use rustls::{client::verify_server_cert_signed_by_trust_anchor, ServerName};
use std::time::SystemTime;

pub fn ca_cert() -> (Certificate, String, String) {
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

    let keypair = KeyPair::generate(&PKCS_ECDSA_P256_SHA256).unwrap();
    params.key_pair.replace(keypair);

    let mut dn = DistinguishedName::new();
    dn.push(DnType::OrganizationName, "JLer");
    dn.push(
        DnType::CommonName,
        DnValue::PrintableString("JLer-CA".to_string()),
    );
    params.distinguished_name = dn;

    let cert = Certificate::from_params(params).unwrap();
    let cert_pem = cert.serialize_pem_with_signer(&cert).unwrap();
    let key_pem = cert.serialize_private_key_pem();
    (cert, cert_pem, key_pem)
}

pub fn create_csr(domain: &str) -> (String, String) {
    let subject_alt_names = vec![domain.into()];
    let mut params = CertificateParams::new(subject_alt_names);

    let mut dn = DistinguishedName::new();
    dn.push(DnType::OrganizationName, "JLer");
    dn.push(DnType::CommonName, DnValue::PrintableString(domain.into()));
    params.distinguished_name = dn;

    let keypair = KeyPair::generate(&PKCS_ECDSA_P256_SHA256).unwrap();
    params.key_pair.replace(keypair);

    let cert = Certificate::from_params(params).unwrap();

    let csr_pem = cert.serialize_request_pem().unwrap();
    let key_pem = cert.serialize_private_key_pem();

    (csr_pem, key_pem)
}

pub fn restore_ca_cert(ca_cert_pem: &str, ca_key_pem: &str) -> Certificate {
    let ca_key_pair = KeyPair::from_pem(ca_key_pem).unwrap();
    let ca_param = CertificateParams::from_ca_cert_pem(ca_cert_pem, ca_key_pair).unwrap();

    Certificate::from_params(ca_param).unwrap()
}

pub fn sign_csr(csr_pem: &str, ca_cert: &Certificate) -> String {
    let csr = CertificateSigningRequest::from_pem(csr_pem).unwrap();
    csr.serialize_pem_with_signer(ca_cert).unwrap()
}

pub fn read_ca(ca_path: &PathBuf) -> Result<RootCertStore> {
    let ca_certs = read_certs(ca_path)?;
    let mut roots = rustls::RootCertStore::empty();
    for cert in ca_certs {
        roots.add(&cert)?;
    }
    Ok(roots)
}

pub fn read_certs(cert_path: &PathBuf) -> Result<Vec<rustls::Certificate>> {
    let cert_chain = fs::read(cert_path).context("failed to read certificate chain")?;
    let cert_chain = if cert_path.extension().map_or(false, |x| x == "der") {
        vec![rustls::Certificate(cert_chain)]
    } else {
        rustls_pemfile::certs(&mut &*cert_chain)
            .context("invalid PEM-encoded certificate")?
            .into_iter()
            .map(rustls::Certificate)
            .collect()
    };

    Ok(cert_chain)
}

pub fn read_key(key_path: &PathBuf) -> Result<rustls::PrivateKey> {
    let key = fs::read(key_path).context("failed to read private key")?;
    let key = if key_path.extension().map_or(false, |x| x == "der") {
        rustls::PrivateKey(key)
    } else {
        let pkcs8 = rustls_pemfile::pkcs8_private_keys(&mut &*key)
            .context("malformed PKCS #8 private key")?;
        match pkcs8.into_iter().next() {
            Some(x) => rustls::PrivateKey(x),
            None => {
                let rsa = rustls_pemfile::rsa_private_keys(&mut &*key)
                    .context("malformed PKCS #1 private key")?;
                match rsa.into_iter().next() {
                    Some(x) => rustls::PrivateKey(x),
                    None => {
                        bail!("no private keys found");
                    }
                }
            }
        }
    };

    Ok(key)
}

/// `ServerCertVerifier` that verifies that the server is signed by a trusted root, but allows any serverName
/// see the trait impl for more information.
pub struct WebPkiVerifierAnyServerName {
    roots: RootCertStore,
}

impl WebPkiVerifierAnyServerName {
    /// Constructs a new `WebPkiVerifierAnyServerName`.
    ///
    /// `roots` is the set of trust anchors to trust for issuing server certs.
    pub fn new(roots: RootCertStore) -> Self {
        Self { roots }
    }
}

impl ServerCertVerifier for WebPkiVerifierAnyServerName {
    /// Will verify the certificate is valid in the following ways:
    /// - Signed by a  trusted `RootCertStore` CA
    /// - Not Expired
    fn verify_server_cert(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        now: SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let cert = ParsedCertificate::try_from(end_entity)?;
        verify_server_cert_signed_by_trust_anchor(&cert, &self.roots, intermediates, now)?;
        Ok(ServerCertVerified::assertion())
    }
}

#[tokio::test]
async fn test() {
    use crate::file::{create_file, read_file_to_string};
    // ca
    let (_, ca_cert_pem, ca_key_pem) = ca_cert();
    create_file("./config/cert/ca_cert.pem", ca_cert_pem.as_bytes())
        .await
        .unwrap();
    create_file("./config/cert/ca_key.pem", ca_key_pem.as_bytes())
        .await
        .unwrap();

    // server csr
    let (csr_pem, key_pem) = create_csr("test-host");
    create_file("./config/cert/server_csr.pem", csr_pem.as_bytes())
        .await
        .unwrap();
    create_file("./config/cert/server_key.pem", key_pem.as_bytes())
        .await
        .unwrap();
    // server sign
    let ca_cert_pem = read_file_to_string("./config/cert/ca_cert.pem")
        .await
        .unwrap();
    let ca_key_pem = read_file_to_string("./config/cert/ca_key.pem")
        .await
        .unwrap();
    let ca = restore_ca_cert(&ca_cert_pem, &ca_key_pem);
    let csr_pem = read_file_to_string("./config/cert/server_csr.pem")
        .await
        .unwrap();
    let cert_pem = sign_csr(&csr_pem, &ca);
    create_file("./config/cert/server_cert.pem", cert_pem.as_bytes())
        .await
        .unwrap();

    // client csr
    let (csr_pem, key_pem) = create_csr("client.test-host");
    create_file("./config/cert/client_csr.pem", csr_pem.as_bytes())
        .await
        .unwrap();
    create_file("./config/cert/client_key.pem", key_pem.as_bytes())
        .await
        .unwrap();
    // client sign
    let ca_cert_pem = read_file_to_string("./config/cert/ca_cert.pem")
        .await
        .unwrap();
    let ca_key_pem = read_file_to_string("./config/cert/ca_key.pem")
        .await
        .unwrap();
    let ca = restore_ca_cert(&ca_cert_pem, &ca_key_pem);
    let csr_pem = read_file_to_string("./config/cert/client_csr.pem")
        .await
        .unwrap();
    let cert_pem = sign_csr(&csr_pem, &ca);
    create_file("./config/cert/client_cert.pem", cert_pem.as_bytes())
        .await
        .unwrap();
}
