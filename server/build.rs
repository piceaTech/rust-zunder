use std::env;

fn main(){
  let domain = env::var("ZUNDER_DOMAIN").expect("Please specify a Domain (ZUNDER_DOMAIN). See https://github.com/piceaTech/rust-zunder#server");
  println!("cargo:rerun-if-env-changed=ZUNDER_DOMAIN");
  println!("cargo:rustc-env=ZUNDER_DOMAIN={}", domain)
}
