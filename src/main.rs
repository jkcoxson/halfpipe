// Jackson Coxson

use std::{
    io::{self, Write},
    net::{SocketAddr, SocketAddrV4},
    str::FromStr,
    sync::Arc,
};

use quinn::crypto::rustls::QuicClientConfig;
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    let mut roots = rustls::RootCertStore::empty();
    let path = std::env::current_dir().unwrap();
    let cert_path = path.join("keys/ca/ca.crt");
    let cert = std::fs::read_to_string(cert_path).unwrap();
    roots
        .add(
            CertificateDer::pem_slice_iter(cert.as_bytes())
                .next()
                .unwrap()
                .unwrap(),
        )
        .unwrap();

    let cert_path = path.join("keys/client/cert.pem");
    let key_path = path.join("keys/client/key.pem");
    let (cert, key) =
        match std::fs::read(&cert_path).and_then(|x| Ok((x, std::fs::read(&key_path)?))) {
            Ok((cert, key)) => (
                CertificateDer::pem_slice_iter(&cert)
                    .next()
                    .unwrap()
                    .unwrap(),
                PrivateKeyDer::pem_slice_iter(&key).next().unwrap().unwrap(),
            ),
            Err(e) => {
                panic!("{:?}", e);
            }
        };

    let mut client_crypto = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(vec![cert], key)
        .unwrap();

    client_crypto.alpn_protocols = [b"hq-29"].iter().map(|&x| x.into()).collect();

    let client_config =
        quinn::ClientConfig::new(Arc::new(QuicClientConfig::try_from(client_crypto).unwrap()));
    let mut endpoint =
        quinn::Endpoint::client(SocketAddr::V4(SocketAddrV4::from_str("0.0.0.0:0").unwrap()))?;
    endpoint.set_default_client_config(client_config);

    let host = "halfpipe.jkcoxson.com";
    let remote = SocketAddr::V4(SocketAddrV4::from_str("127.0.0.1:4444").unwrap());
    eprintln!("connecting to {host} at {remote}");
    let conn = endpoint
        .connect(remote, host)
        .unwrap()
        .await
        .map_err(|e| panic!("failed to connect: {}", e))
        .unwrap();
    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|e| panic!("failed to open stream: {}", e))
        .unwrap();

    send.write_all("why hello there".as_bytes())
        .await
        .map_err(|e| panic!("failed to send request: {}", e))
        .unwrap();
    send.finish().unwrap();
    let resp = recv
        .read_to_end(usize::MAX)
        .await
        .map_err(|e| panic!("failed to read response: {}", e))
        .unwrap();
    io::stdout().write_all(&resp).unwrap();
    io::stdout().flush().unwrap();
    conn.close(0u32.into(), b"done");

    // Give the server a fair chance to receive the close packet
    endpoint.wait_idle().await;

    Ok(())
}
