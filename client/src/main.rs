use std::process::exit;
use std::error::Error;
use std::env;
use std::process::Command;

// use std::fs::File;
// use std::io::prelude::*;



use std::io::SeekFrom;
use std::fs::OpenOptions;
use std::io::prelude::*;



use serde::{Deserialize, Serialize};



fn main() -> Result<(), Box<Error>>{
  let mut file = OpenOptions::new().read(true).write(true).create(true).open("./.zunder.toml")?;

  let args: Vec<String> = env::args().collect();
  if args.len() > 1 && args[1] == "create"{
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    if contents.len() > 0 && !(args.len() > 2 && args[2] == "--force"){
      println!("Config already existing.");
      println!("To overwrite use: zunder create --force");
      exit(1);
    }
    let config = Config {
      localport: 8080,
      remoteport: None,
      subdomain: "blog".to_string(),
      ssh: "ssh-server".to_string()
    };
    let to_save = toml::to_string(&config).unwrap();
    file.seek(SeekFrom::Start(0))?; // default is to write at the end of the file
    file.set_len(0)?; // remove all content which was there before
    file.write_all(to_save.as_bytes())?; // write new template into this file
    file.sync_all()?;
    println!("Wrote default config.");
    exit(0);
  }
  if args.len() > 1 && args[1] == "help"{
    println!("zunder: remote-connect to dev");
    println!("Usage:");
    println!("\tzunder (serve the current config file.)");
    println!("\tzunder create (create a default config in the current folder.)");
    exit(0);
  }

  // read config from local file
    
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    if contents.len() == 0{
      println!("Config does not exist. Create a local config by running: zunder create");
      exit(1);
    }
    let mut config = match toml::from_str::<Config>(&contents){
      Err(_) => {
        println!("Config is not valid.");
        println!("Maybe recreate a config by running: zunder create");
        exit(1);
      },
      Ok(exp) => exp
    };

    
    // find port which is available
    if config.remoteport.is_none(){
      config.remoteport = Some(ssh_get_avail_port(&config)?);
    }
    else{
      if !check_remote_port(&config)?{
        config.remoteport = Some(ssh_get_avail_port(&config)?);
      }
    }

    // save config
    
    let to_save = toml::to_string(&config).unwrap();
    file.seek(SeekFrom::Start(0))?; // default is to write at the end of the file
    file.set_len(0)?; // remove all content which was there before
    file.write_all(to_save.as_bytes())?; // write new template into this file
    file.sync_all()?;

    
    // server create config
    if !server_create_site(&config)?{
      println!("Could not create Site on server for the config: {:?}", config);
      exit(8);
    }
    
    println!("Waiting for connections...");
    execute_command(&ssh_forward_string(&config))?;

    Ok(())
}


fn execute_command(arg: &str)-> Result<String, Box<Error>> {
  let output = if cfg!(target_os = "windows") {
    Command::new("cmd")
            .args(&["/C", arg])
            .output()
            .expect("failed to execute process")
} else {
    Command::new("sh")
            .arg("-c")
            .arg(arg)
            .output()
            .expect("failed to execute process")
    };
  Ok(String::from(std::str::from_utf8(&output.stdout)?))
}

fn ssh_forward_string(config: &Config) -> String {
  format!("ssh -NCR {remoteport}:localhost:{localport} {ssh}", remoteport=config.remoteport.unwrap(), localport=config.localport, ssh=config.ssh)
}

fn server_create_site(config: &Config) -> Result<bool, Box<Error>>{
  let cmd = format!("ssh {ssh} \"/usr/bin/zunder-server create {subdomain} {remoteport}\"", ssh=config.ssh, subdomain=config.subdomain, remoteport=config.remoteport.unwrap());
  let s = execute_command(&cmd)?;
  return Ok(s.trim() == "Successfully created everything.");
}

fn check_remote_port(config: &Config) -> Result<bool, Box<Error>>{
  let cmd = format!("ssh {ssh} \"/usr/bin/zunder-server test {port}\"", ssh=config.ssh, port=config.remoteport.unwrap());
  let s = execute_command(&cmd)?;
  return Ok(s.trim() == "free");
}

fn ssh_get_avail_port(config: &Config) -> Result<u16, Box<Error>>{

  let cmd = format!("ssh {ssh} \"/usr/bin/zunder-server find_port\"", ssh=config.ssh);
  let s = execute_command(&cmd)?;
  let port: u16 = s.trim().parse()?;
  Ok(port)
}


#[derive(Debug, Deserialize, Serialize)]
struct Config {
	localport: u16,
  remoteport: Option<u16>,
  subdomain: String,
  ssh: String
}