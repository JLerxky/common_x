use std::sync::Arc;
use std::{fs, path::PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, DnValue, ExtendedKeyUsagePurpose,
    IsCa, KeyPair, KeyUsagePurpose,
};
use rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use rustls::client::verify_server_cert_signed_by_trust_anchor;
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, PrivateSec1KeyDer, ServerName, UnixTime};
use rustls::server::ParsedCertificate;
use rustls::{ClientConfig, RootCertStore};
use webpki::ALL_VERIFICATION_ALGS;

pub fn new_ca() -> (Certificate, KeyPair) {
    let mut params = CertificateParams::new(Vec::default()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.distinguished_name.push(
        DnType::CountryName,
        DnValue::PrintableString("CN".try_into().unwrap()),
    );
    params
        .distinguished_name
        .push(DnType::OrganizationName, "JLer");
    params
        .distinguished_name
        .push(DnType::CommonName, "JLer-CA");
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params.key_usages.push(KeyUsagePurpose::KeyCertSign);
    params.key_usages.push(KeyUsagePurpose::CrlSign);

    let key_pair = KeyPair::generate().unwrap();
    (params.self_signed(&key_pair).unwrap(), key_pair)
}

pub fn new_end_entity(name: &str, ca: &Certificate, ca_key: &KeyPair) -> (Certificate, KeyPair) {
    let mut params = CertificateParams::new(vec![name.into()]).unwrap();
    params.distinguished_name.push(DnType::CommonName, name);
    params.use_authority_key_identifier_extension = true;
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ServerAuth);

    let key_pair = KeyPair::generate().unwrap();
    (params.signed_by(&key_pair, ca, ca_key).unwrap(), key_pair)
}

pub fn read_ca(ca_path: String) -> Result<RootCertStore> {
    let ca_certs = read_certs(ca_path)?;
    let mut roots = rustls::RootCertStore::empty();
    for cert in ca_certs {
        roots.add(cert)?;
    }
    Ok(roots)
}

pub fn read_certs(cert_path: String) -> Result<Vec<rustls::pki_types::CertificateDer<'static>>> {
    let cert_bytes = fs::read(&cert_path).context("failed to read certificate chain")?;

    let cert_chain = if Into::<PathBuf>::into(cert_path)
        .extension()
        .is_some_and(|x| x == "der")
    {
        vec![CertificateDer::from(cert_bytes)]
    } else {
        rustls_pemfile::certs(&mut &*cert_bytes).collect::<Result<Vec<_>, _>>()?
    };

    Ok(cert_chain)
}

pub fn read_key(key_path: String) -> Result<rustls::pki_types::PrivateKeyDer<'static>> {
    let key = fs::read(&key_path).context("failed to read private key")?;
    let key = if Into::<PathBuf>::into(key_path)
        .extension()
        .is_some_and(|x| x == "der")
    {
        PrivateSec1KeyDer::from(key).into()
    } else {
        let pkcs8 =
            rustls_pemfile::pkcs8_private_keys(&mut &*key).collect::<Result<Vec<_>, _>>()?;
        match pkcs8.into_iter().next() {
            Some(x) => x.into(),
            None => {
                let rsa =
                    rustls_pemfile::rsa_private_keys(&mut &*key).collect::<Result<Vec<_>, _>>()?;
                match rsa.into_iter().next() {
                    Some(x) => x.into(),
                    None => {
                        bail!("no private keys found");
                    }
                }
            }
        }
    };

    Ok(key)
}

impl ServerCertVerifier for WebPkiVerifierAnyServerName {
    /// Will verify the certificate is valid in the following ways:
    /// - Signed by a  trusted `RootCertStore` CA
    /// - Not Expired
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let cert = ParsedCertificate::try_from(end_entity)?;
        verify_server_cert_signed_by_trust_anchor(
            &cert,
            &self.roots,
            intermediates,
            now,
            ALL_VERIFICATION_ALGS,
        )?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// `ServerCertVerifier` that verifies that the server is signed by a trusted root, but allows any serverName
/// see the trait impl for more information.
#[derive(Debug)]
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

pub fn create_any_server_name_config(ca_path: &str) -> Result<ClientConfig> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();
    Ok(ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(WebPkiVerifierAnyServerName::new(read_ca(
            ca_path.to_owned(),
        )?)))
        .with_no_client_auth())
}

#[tokio::test]
async fn test() {
    use crate::file::create_file;
    // ca
    let (ca_cert, ca_key_pair) = new_ca();
    create_file("./config/cert/ca_cert.pem", ca_cert.pem().as_bytes())
        .await
        .unwrap();
    create_file(
        "./config/cert/ca_key.pem",
        ca_key_pair.serialize_pem().as_bytes(),
    )
    .await
    .unwrap();

    // server cert
    let (server_cert, server_key) = new_end_entity("test-host", &ca_cert, &ca_key_pair);
    create_file(
        "./config/cert/server_cert.pem",
        server_cert.pem().as_bytes(),
    )
    .await
    .unwrap();
    create_file(
        "./config/cert/server_key.pem",
        server_key.serialize_pem().as_bytes(),
    )
    .await
    .unwrap();

    // client cert
    let (client_cert, client_key) = new_end_entity("client.test-host", &ca_cert, &ca_key_pair);
    create_file(
        "./config/cert/client_cert.pem",
        client_cert.pem().as_bytes(),
    )
    .await
    .unwrap();
    create_file(
        "./config/cert/client_key.pem",
        client_key.serialize_pem().as_bytes(),
    )
    .await
    .unwrap();
}
