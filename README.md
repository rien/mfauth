# MFAuth

_Multi Factor Authentication for CLI mail clients_

## Installation

1. Install Rust, either with your package manager or using [rustup](https://rustup.rs/)
2. Clone this repository and enter it
  ```
  git@github.com:rien/mfauth.git
  cd mfauth
  ```
3. Build and install the program
  ```
  cargo install --path .
  ```
4. Test if it works by running `mfauth help`.

If you see a `command not found` error, you can try putting `$HOME/.cargo/bin`  in your `$PATH`.

## Usage

1. Run `mfauth init` to create an example configuration file in `$HOME/.config/mfauth/config.toml`
2. Edit the configuration file
3. Run `mfauth authorize <account>` to get a valid session (an access and refresh token), this will save this session in `$HOME/.cache/mfauth/cache.toml`
4. Run `mfauth access <account>` to get a valid access token, this will automatically refresh tokens if needed
5. Configure your mail client to use `XOAUTH2` authentication and use mfauth to fetch the access tokens

Once in a while this might stop working if your session expires. If that happens, you can simply run `mfauth authrorize <account>` again and you're good to go!


### Example with msmtp (sending)

A working account entry in the msmtp config using mfauth:

```
account microsoft
auth xoauth2
from username@outlook.com
host outlook.office365.com
passwordeval /home/user/.cargo/bin/mfauth access microsoft
port 587
tls on
tls_starttls on
tls_trust_file /etc/ssl/certs/ca-certificates.crt
user username@outlook.com
```

### Example with mbsync (receiving)

**Important:** isync requires the [`cyrus-sasl-xoauth2`](https://github.com/moriyoshi/cyrus-sasl-xoauth2) extension installed to support the `XOAUTH2` authentication protocol.

This is an example of a working `IMAPAccount` block using mfauth.

```
IMAPAccount microsoft
AuthMechs XOAUTH2
CertificateFile /etc/ssl/certs/ca-certificates.crt
Host outlook.office365.com
PassCmd "/home/user/.cargo/bin/mfauth access microsoft"
Port 993
SSLType IMAPS
User username@outlook.com
```

