use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair,
};
use time::{Duration, OffsetDateTime};

const ROOT_HOSTNAME: &str = "trustedroot.esta.org";
const COUNTRY: &str = "US";
const STATE: &str = "Illinois";
const LOCALITY: &str = "Chicago";
const ORGANIZATION: &str = "Next-Gen Transport Task Group";

fn main() {
    let now = OffsetDateTime::now_utc();
    let expiry = now + Duration::days(365);

    std::fs::create_dir_all("out").unwrap();

    let (root_cert, root_key) = create_root_cert(&now, &expiry);
    create_signed_cert(&root_cert, &root_key, "arbiter", &now, &expiry);
    create_signed_cert(&root_cert, &root_key, "client", &now, &expiry);
}

// Equivalent OpenSSL command:
// openssl req -x509 -nodes -days 365 -newkey ed25519 -keyout root-key.pem -out root-cert.pem
fn create_root_cert(now: &OffsetDateTime, expiry: &OffsetDateTime) -> (Certificate, KeyPair) {
    let mut cert_params = CertificateParams::new(vec![ROOT_HOSTNAME.to_string()]).unwrap();
    update_dn(&mut cert_params.distinguished_name, ROOT_HOSTNAME);
    cert_params.not_before = now.clone();
    cert_params.not_after = expiry.clone();
    cert_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

    let key_pair = KeyPair::generate().unwrap();
    std::fs::write("out/root-key.pem", key_pair.serialize_pem()).unwrap();
    let cert = cert_params.self_signed(&key_pair).unwrap();
    std::fs::write("out/root-cert.pem", &cert.pem()).unwrap();

    (cert, key_pair)
}

// Equivalent OpenSSL commands:
// openssl req -new -nodes -newkey ed25519 -keyout [component-name]-key.pem -out [component-name]-req.csr
// openssl x509 -req -in [component-name]-req.csr -days 365 -CA root-cert.pem -CAkey root-key.pem -CAcreateserial -out [component-name]-cert.pem
fn create_signed_cert(
    root_cert: &Certificate,
    root_key: &KeyPair,
    component_name: &str,
    now: &OffsetDateTime,
    expiry: &OffsetDateTime,
) {
    let hostname = format!("{component_name}.local");
    let mut cert_params = CertificateParams::new(vec![hostname.clone()]).unwrap();
    update_dn(&mut cert_params.distinguished_name, &hostname);
    cert_params.not_before = now.clone();
    cert_params.not_after = expiry.clone();

    let key_pair = KeyPair::generate().unwrap();
    std::fs::write(
        format!("out/{component_name}-key.pem"),
        key_pair.serialize_pem(),
    )
    .unwrap();
    let cert = cert_params
        .signed_by(&key_pair, root_cert, root_key)
        .unwrap();
    std::fs::write(format!("out/{component_name}-cert.pem"), &cert.pem()).unwrap();
}

fn update_dn(dn: &mut DistinguishedName, cn: &str) {
    dn.push(DnType::CommonName, cn);
    dn.push(DnType::CountryName, COUNTRY);
    dn.push(DnType::StateOrProvinceName, STATE);
    dn.push(DnType::LocalityName, LOCALITY);
    dn.push(DnType::OrganizationName, ORGANIZATION);
}
