// Jackson Coxson
// Replicates the QUIC tunnel on a device

use std::io::Read;

use std::str::FromStr;
use std::{fs, sync::Arc};

use quinn::crypto::rustls::QuicServerConfig;
use quinn::{ConnectionError, Incoming};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::RootCertStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    run().await.unwrap();
    let mut config = tun::Configuration::default();
    config
        .address((10, 0, 18, 20))
        .netmask((255, 255, 255, 0))
        .destination((10, 0, 18, 1))
        .up();

    #[cfg(target_os = "linux")]
    config.platform_config(|config| {
        // requiring root privilege to acquire complete functions
        config.ensure_root_privileges(true);
    });

    let mut dev = tun::create(&config).unwrap();
    let mut buf = [0; 4096];

    loop {
        let amount = dev.read(&mut buf).unwrap();
        println!("{:?}", &buf[0..amount]);
    }
}

async fn run() -> Result<(), ()> {
    let path = std::env::current_dir().unwrap();
    let cert_path = path.join("keys/server/cert.pem");
    let key_path = path.join("keys/server/key.pem");
    let (cert, key) = match fs::read(&cert_path).and_then(|x| Ok((x, fs::read(&key_path)?))) {
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

    let (certs, key) = (vec![cert], key);

    let root_cert_path = path.join("keys/ca/ca.crt");
    let root_key_path = path.join("keys/ca/ca.key");
    let (root_cert, _root_key) =
        match fs::read(&root_cert_path).and_then(|x| Ok((x, fs::read(&root_key_path)?))) {
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

    let mut store = RootCertStore::empty();
    store.add(root_cert).unwrap();
    let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(store))
        .build()
        .unwrap();

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)
        .unwrap();
    server_crypto.alpn_protocols = [b"hq-29"].iter().map(|&x| x.into()).collect();

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
        QuicServerConfig::try_from(server_crypto).unwrap(),
    ));
    let transport_config = Arc::get_mut(&mut server_config.transport).unwrap();
    transport_config.max_concurrent_uni_streams(0_u8.into());

    let endpoint = quinn::Endpoint::server(
        server_config,
        std::net::SocketAddr::V4(std::net::SocketAddrV4::from_str("0.0.0.0:4444").unwrap()),
    )
    .unwrap();
    eprintln!("listening on {}", endpoint.local_addr().unwrap());

    while let Some(conn) = endpoint.accept().await {
        println!("accepting connection");
        let fut = handle_connection(conn);
        tokio::spawn(async move {
            if let Err(e) = fut.await {
                println!("connection failed: {reason}", reason = e)
            }
        });
    }

    Ok(())
}

async fn handle_connection(conn: Incoming) -> Result<(), ConnectionError> {
    let conn = conn.await.unwrap();
    loop {
        let stream = conn.accept_bi().await;
        let (mut tx, mut rx) = match stream {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                println!("connection closed");
                return Ok(());
            }
            Err(e) => {
                return Err(e);
            }
            Ok(s) => s,
        };

        let res = rx.read_to_end(64 * 1000).await.unwrap();
        tx.write(&res).await.unwrap();
    }
}
