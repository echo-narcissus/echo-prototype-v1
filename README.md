# ECHO 

## Setup (POSIX SYSTEMS)
First, build echo with either of the following options:
```bash
make build
```
or
```bash
cd echo 
cargo build -r 
cd ..
mkdir bin/
cp echo/target/release/echo ./bin/echo-server
```

Second, create your TLS certificate and key with either of the following options:
```bash
make tls
```
or
```bash
mkdir keys/
OPENSSL_CONF=/etc/ssl/openssl.cnf \
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout keys/key.pem -out keys/cert.pem -days 365 \
  -subj "/CN=echo-server"
```

**The makefile scripts do the exact same thing, they're just slightly more convenient**

You can do both of these steps in one through the following command:
```bash
make setup
```

## Running the server (POSIX SYSTEMS)

you can start the default server with:
```bash
./bin/echo-server -k ./keys/key.pem -c ./keys/cert.pem
```
or
```bash
make start_default
```
which will automatically build echo and create keys if needed.

Clean up the current binary, keys, and cargo artifacts with
```bash
make clean
```


## Options and Usage:

Usage: echo OPTIONS --cert CERT --key KEY

|       Flags       |  Arguments  |  Default  |
| ----------------- | ----------- | --------- |
| -c, --cert        | CERT        |           |
| -k, --key         | KEY         |           |
| -p, --port        | PORT        |    4433   |
| -b, --bind-addr   | BIND_ADDR   | 127.0.0.1 |
| -m, --msg-id-size | MSG_ID_SIZE |     64    |
| -h, --help        |           |           |
| -V, --version     |           |           |

