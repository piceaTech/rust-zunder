# Rust - Zunder

## Installation

### Server

First of all you have to create the server executable, upload that to the server and allow it to change the config of nginx.

1. Compile: `ZUNDER_DOMAIN=your-domain.dev cargo rustc --release --target x86_64-unknown-linux-musl -- -C linker=rust-lld`

2. Copy: `scp target/x86_64-unknown-linux-musl/release/zunder-server server:/usr/bin`

3. Make Sticky: `chown root zunder-server && chmod u+s zunder-server`

4. Create a wildcard-certificate for your domain with let's encrypt.

### Client

1. `cargo install zunder`
2. Run `zunder create` in a project folder which you want to forward
3. Edit your `.zunder.toml` to change the port you want to forward, where the server can be reached and which subdomain should be used.
4. Run `zunder`
5. It should now be connected to the server an create the subdomain for nginx automatically and proxy all your requests. 


## Usage

**Don't use this in production as it is only a little more than a proof of concept.**

Here is a config I use for a project using ember (hence the port 4200):
```shell
localport = 4200
remoteport = 57407
subdomain = "pp"
ssh = "os"
```
