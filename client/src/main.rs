use std::time::Duration;
use std::process::exit;
use std::error::Error;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
// use std::process::Command;

// use std::fs::File;
// use std::io::prelude::*;
use std::net::{TcpStream};
use std::{thread, time};

use std::io::SeekFrom;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::ErrorKind;


use serde::{Deserialize, Serialize};

use ssh2::{Channel, Listener, Session};


static WAIT_TIME: Duration = time::Duration::from_millis(100);
static WAIT_TIME_SHORT: Duration = time::Duration::from_millis(3);

fn main() -> Result<(), Box<Error>>{
  let mut file = OpenOptions::new().read(true).write(true).create(true).open("./.zunder.toml")?;
  let args: Vec<String> = env::args().collect();
  if args.len() > 1 && args[1] == "create"{
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

  // read config from local file
    
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let mut config = toml::from_str::<Config>(&contents).expect("Config is not valid.");

    // create config on the server
    
    // big thanks to https://marianafranco.github.io/2017/03/10/libssh2-tunnel/
    let mut sess = Session::new().unwrap();





    let tcp = TcpStream::connect(&config.ssh).unwrap();
    sess.set_compress(true);

    sess.handshake(&tcp).unwrap();



    use std::path::Path;
    sess.userauth_pubkey_file("spruce", Some(Path::new("/Users/spruce/.ssh/id_rsa.pub")), Path::new("/Users/spruce/.ssh/id_rsa"), None).unwrap();
    // sess.userauth_agent("spruce").unwrap();
    
    // find port which is available
    if config.remoteport.is_none(){
      config.remoteport = Some(get_available_remote_port(&sess)?);
    }
    else{
      if !check_remote_port(&sess, config.remoteport.unwrap())?{
        config.remoteport = Some(get_available_remote_port(&sess)?);   
      }
    }

    // save config
    let to_save = toml::to_string(&config).unwrap();
    file.seek(SeekFrom::Start(0))?; // default is to write at the end of the file
    file.set_len(0)?; // remove all content which was there before
    file.write_all(to_save.as_bytes())?; // write new template into this file
    file.sync_all()?;

    
    // server create config
    if !server_create_site(&sess, &config)?{
      println!("Could not create Site on server for the config: {:?}", config);
      exit(8);
    }
    
    
    let (mut listener, port) = sess.channel_forward_listen(config.remoteport.unwrap(), Some("localhost"), None)?;
    if port != config.remoteport.unwrap(){
      println!("Port should be {} but was {}", config.remoteport.unwrap(), port);
      exit(1);
    }
    sess.set_blocking(false);
    println!("Waiting for connections ...");

    let mut current_streams = Vec::new();
    loop{
      let new_connection = new_connection(&mut listener, &sess, config.localport, current_streams.len() == 0);
      if new_connection.is_ok(){
        let connection = new_connection.unwrap();
        current_streams.push(connection);
      }
      current_streams.retain(|x| x.active);
      let mut is_block = true;
      let all_ids: Vec<_> = current_streams.iter().map(|item| item.time).collect();
      for mut connection in &mut current_streams {
        if connection.remote.eof(){
          println!("EOF: {:?},", connection.time);
          println!("{:?}", all_ids);
          is_block &= write_local_to_remote(connection)?;

          connection.active = false;
          // continue;
        }
        is_block &= write_local_to_remote(connection)?;
        is_block &= write_remote_to_local(connection)?;
        
      }
      println!("current, {:?}", current_streams.len());
      if is_block{
        // println!("Waiting, {}", current_streams.len());
        thread::sleep(WAIT_TIME);
        print!(".{}|", current_streams.len());
      }
      else{
        print!("-{}/", current_streams.len());
        // thread::sleep(WAIT_TIME_SHORT);
      }
      std::io::stdout().flush()?;
        



    }
    // connect ssh
    // Ok(())
}


fn new_connection<'listener>(listener: &mut Listener<'listener>, sess: &'listener Session, localport: u16, block: bool) -> Result<ProxyConnection<'listener>, Box<Error>>{
  if block{
    sess.set_blocking(true);
  }
  let channel = match listener.accept() {
    Ok(ch) => ch,
    Err(_) => {
      if block{
        sess.set_blocking(false);
      }
      return Err(Box::new(std::io::Error::new(ErrorKind::Other, "no new connection waiting.")))
    }
    ,
  };
  if block{
    sess.set_blocking(false);
  }
  println!("Got a new connection.");
  let local_stream = TcpStream::connect(format!("127.0.0.1:{}", localport)).expect("Can connect to local server");
  local_stream.set_nonblocking(true)?;
  return Ok(ProxyConnection{
    time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_millis() - 1565718121181 - 15444721 - 1014595,
    session: &sess,
    remote: channel,
    local: local_stream,
    remote_buf: Box::new([0;1000]),
    remote_amount: 0,
    remote_written: 0,
    buf_to_local: Box::new([0;10000]),
    amount_to_local: 0,
    active: true,
  });
}
//remote_channel: &mut Channel, local: &mut TcpStream
fn write_remote_to_local(conn: &mut ProxyConnection) -> Result<bool, Box<Error>>{
  if conn.amount_to_local == 0{
    conn.amount_to_local = match conn.remote.read(&mut conn.buf_to_local){
      Err(err) => {
        // println!("{:?}", err.into_inner().unwrap().to_string() != "[-37] would block");
        let err_string = err.into_inner().unwrap().to_string();
        if err_string == "[-37] would block"{
          return Ok(true);
        }
        else{
          println!("remote err {:?}", err_string);
          println!("remote err aufgetreten");
          exit(3);
        }
      },
      Ok(expr) => expr,
    };
    // hier vielleicht noch ne schleife
    // println!("bytes_remote: {:?}", bytes_remote);
    if conn.amount_to_local > 0{
      let written = match conn.local.write_all(&conn.buf_to_local[0 .. conn.amount_to_local]){
        Err(_) => {
          println!("remote break");
          0
        },
        Ok(_) => conn.amount_to_local
      };
      conn.amount_to_local -= written;
    }
  }
  Ok(false)
}

pub struct ProxyConnection<'con> {
  pub time: u128,
  pub local: TcpStream,
  pub remote: Channel<'con>,
  pub session: &'con Session,
  pub remote_buf: Box<[u8]>,
  pub remote_amount: usize,
  pub remote_written: usize,
  pub buf_to_local: Box<[u8]>,
  pub amount_to_local: usize,
  pub active: bool,
}

// local: &mut TcpStream, remote_channel: &mut Channel, sess: &Session
fn write_local_to_remote(conn: &mut ProxyConnection) -> Result<bool, Box<Error>>{
  if conn.remote_amount == conn.remote_written {
    conn.remote_written = 0;
    conn.remote_amount = match conn.local.read(&mut conn.remote_buf){
      Err(err) => {
        if err.kind() == ErrorKind::WouldBlock{
          return Ok(true);
        }
        else{
          println!("local err2 {:?}", err);
          return Ok(false);
        }
      }
      Ok(expr) => expr,
    }
  }

  if conn.remote_amount > conn.remote_written{
    println!("1 local break {}: {} - {}",  conn.time, conn.remote_amount,conn.remote_written);
          // let read = conn.remote.read_window();
          // println!("1 read: {:?} {:?} {:?}", read.remaining, read.available, read.window_size_initial);
          // let write = conn.remote.write_window();
          // println!("1 write1: {:?} {:?}", write.remaining, write.window_size_initial);
          println!("1 {:?}", ssh2::Error::last_error(&conn.session).unwrap().code());
    let written = match conn.remote.write(&conn.remote_buf[conn.remote_written .. conn.remote_amount]) {
      Err(err) => {
        if ssh2::Error::last_error(&conn.session).unwrap().code() == -37{
          println!("\n37,{},", &conn.time);
          return Ok(true);
        }
        // passiert hier, wenn wir zu schnell packete senden
          // an der stelle: https://github.com/libssh2/libssh2/blob/bc564e9167aa9a1c1d5928df19d07e51b77a47e6/src/channel.c#L2192
          // https://github.com/libssh2/libssh2/blob/934537c449ef6d46dc1991dfa04f85658f4695cf/src/transport.c#L640

        else if ssh2::Error::last_error(&conn.session).unwrap().code() == -39{
          println!("\n39,{},", &conn.time);
          println!("local err: {:?}", err);
        //   println!("local break, {}", conn.remote_amount);
        //   let read = conn.remote.read_window();
        //   println!("read: {:?} {:?} {:?}", read.remaining, read.available, read.window_size_initial);
        //   let write = conn.remote.write_window();
        //   println!("write1: {:?} {:?}", write.remaining, write.window_size_initial);

          return Ok(true);
        }
        else{
          println!("local err: {:?}", err);
          println!("local break {}: {} - {}",  conn.time, conn.remote_amount,conn.remote_written);
          // let read = conn.remote.read_window();
          // println!("read: {:?} {:?} {:?}", read.remaining, read.available, read.window_size_initial);
          // let write = conn.remote.write_window();
          // println!("write1: {:?} {:?}", write.remaining, write.window_size_initial);
          println!("{:?}", ssh2::Error::last_error(&conn.session).unwrap().code());
          exit(2)
        }
      },// no need to close anything, it happens thanks to drop
      Ok(write) => write,
    };
    println!("written: {}", written);
    conn.remote.flush()?;
    conn.remote_written += written;
    
  }
  return Ok(false);
}

fn server_create_site(sess: &Session, config: &Config) -> Result<bool, Box<Error>>{
  let mut channel = sess.channel_session()?;
  channel.exec(&format!("/usr/bin/zunder-server create {} {}", config.subdomain, config.remoteport.unwrap()))?;
  let mut s = String::new();
  channel.read_to_string(&mut s).unwrap();
  return Ok(s.trim() == "Successfully created everything.");
}

fn check_remote_port(sess: &Session, port: u16) -> Result<bool, Box<Error>>{
  let mut channel = sess.channel_session()?;
  channel.exec(&format!("/usr/bin/zunder-server test {}", port))?;
  let mut s = String::new();
  channel.read_to_string(&mut s).unwrap();
  return Ok(s.trim() == "free");
}

fn get_available_remote_port(sess: &Session) -> Result<u16, Box<Error>>{
  let mut channel = sess.channel_session()?;
  channel.exec("/usr/bin/zunder-server find_port")?;
  let mut s = String::new();
  channel.read_to_string(&mut s).unwrap();
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
// fn run_sshto_server(){

// }
// fn create_default_config(){

// }

// let output = if cfg!(target_os = "windows") {
//     Command::new("cmd")
//             .args(&["/C", "echo hello"])
//             .output()
//             .expect("failed to execute process")
// } else {
//     Command::new("sh")
//             .arg("-c")
//             .arg("echo hello")
//             .output()
//             .expect("failed to execute process")
// };