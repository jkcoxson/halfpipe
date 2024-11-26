server:
  cargo build --bin server --features server
  sudo ./target/debug/server

run:
  cargo run --features server

gen-certs:
  echo "Generating root CA certificate"
  mkdir -p keys/ca keys/server keys/client
  openssl genpkey -algorithm RSA -out keys/ca/ca.key -pkeyopt rsa_keygen_bits:4096
  openssl req -x509 -new -nodes -key keys/ca/ca.key -sha256 -days 3650 -out keys/ca/ca.crt -subj "/C=US/ST=UT/L=Provo/O=no/OU=DevOps/CN=DevRootCA"

  echo "Generating server certificate signing request"
  openssl genpkey -algorithm RSA -out keys/server/key.pem -pkeyopt rsa_keygen_bits:4096
  openssl req -new -key keys/server/key.pem -out keys/server/cert.csr -subj "/C=US/ST=UT/L=Provo/O=no/OU=DevOps/CN=halfpipe.jkcoxson.com"

  echo "Creating server extensions file"
  echo "subjectAltName=DNS:halfpipe.jkcoxson.com" > server_exts.conf
  echo "basicConstraints=CA:FALSE" >> server_exts.conf
  echo "keyUsage=digitalSignature,keyEncipherment" >> server_exts.conf
  echo "extendedKeyUsage=serverAuth" >> server_exts.conf

  echo "Signing server certificate with root CA"
  openssl x509 -req -in keys/server/cert.csr -CA keys/ca/ca.crt -CAkey keys/ca/ca.key -CAcreateserial \
    -out keys/server/cert.pem -days 365 -sha256 -extfile server_exts.conf

  echo "Generating client certificate signing request"
  openssl genpkey -algorithm RSA -out keys/client/key.pem -pkeyopt rsa_keygen_bits:4096
  openssl req -new -key keys/client/key.pem -out keys/client/cert.csr -subj "/C=US/ST=UT/L=Provo/O=no/OU=DevOps/CN=DevClient"

  echo "Creating client extensions file"
  echo "basicConstraints=CA:FALSE" > client_exts.conf
  echo "keyUsage=digitalSignature,keyEncipherment" >> client_exts.conf
  echo "extendedKeyUsage=clientAuth" >> client_exts.conf

  echo "Signing client certificate with root CA"
  openssl x509 -req -in keys/client/cert.csr -CA keys/ca/ca.crt -CAkey keys/ca/ca.key -CAcreateserial \
    -out keys/client/cert.pem -days 365 -sha256 -extfile client_exts.conf

  echo "Cleaning up temporary files"
  rm -f server_exts.conf client_exts.conf

  echo "Certificates generated successfully"

