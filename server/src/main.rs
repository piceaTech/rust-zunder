use glob::glob;
use std::path::PathBuf;
use std::env;
use std::process::exit;
use std::net::TcpListener;

use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::SeekFrom;


use std::process::Command;

extern crate regex;
use regex::Regex;

extern crate rand;
use rand::Rng;

fn main() -> Result<(), std::io::Error>{
    let domain_name = env!("ZUNDER_DOMAIN");
		let args: Vec<String> = env::args().collect();
    if args.len() == 1 || (args.len() == 2 && args[1] == "help"){
      println!("This is the server component for zunder.");
      println!("Test: (Testing if a port is available)");
      println!("\tzunder-server test 50000");

      println!("Clean: (Remove all sites from nginx)");
      println!("\tzunder-server clean");

      println!("create: (Create new config in nginx)");
      println!("\tzunder-server create project 50000");
      exit(0);
    }
    else if args.len() == 2 && args[1] == "clean"{
      let sites = list_all_dev_sites()?;
      let amount_sites = sites.len();
      remove_sites(sites)?;
      restart_nginx()?;
      println!("Deleted all sites for {} domain. Removed {} entries.", domain_name, amount_sites);
      exit(0);
    }    
    else if args.len() == 2 && args[1] == "find_port"{
      let mut tests = 0;
      let mut port = 0;
      while tests < 100{
        port = rand::thread_rng().gen_range(50000, 60000);
        check_port(&format!("{}", port));
        tests+=1;
      }
      println!("{}", port);
      exit(0);
    }
    else if args.len() == 3 && args[1] == "test"{
      check_port_exit(&args[2]);

      let listener = TcpListener::bind(format!("127.0.0.1:{}", &args[2]));
      match listener {
        Ok(_expr) => {
          println!("free");
          exit(0);
        },
        Err(_err) => {
          println!("InUse");
          exit(1);
        },
      }
    }
    else if args.len() != 4 || args[1] != "create"{
      println!("Exactly two arguments must be given. There were '{}' arguments present.", args.len() - 1);
      println!("Calling it like: `zunder-server create project 50001`");
      exit(1);
    }

		let sub_domain = &args[2];
		let port = &args[3];

		let template = create_template(sub_domain, &port);
    let filename = format!("/etc/nginx/sites-enabled/{sub_domain}.{domain_name}", sub_domain=sub_domain, domain_name=env!("ZUNDER_DOMAIN"));
    if write_to_filename(&filename, template)?{
      restart_nginx()?;
    }
    println!("Successfully created everything.");
		
		Ok(())
}

fn list_all_dev_sites() -> Result<Vec<PathBuf>, std::io::Error>{
  let mut files: Vec<PathBuf> = Vec::new();
  for e in glob(&format!("/etc/nginx/sites-enabled/*.{}", env!("ZUNDER_DOMAIN"))).expect("Failed to read glob pattern") {
      files.push(e.unwrap());
  }
  Ok(files)

}

fn remove_sites(sites: Vec<PathBuf>) -> Result<(), std::io::Error>{
  if sites.len() == 0{
    return Ok(());
  }
  let status = Command::new("rm").args(sites).status()?;
  match status.code() {
    Some(code) => {
      if code != 0{
        println!("Couldn't remove all dev sites from nginx. Got errorcode '{}'", code);
        exit(7);  
      }
      Ok(())
    }
    None       => {
      println!("Couldn't remove all dev sites from nginx.");
      exit(6);
    }
  }
}


fn write_to_filename(filename: &str, template: String) -> Result<bool,std::io::Error>{
  let mut file = get_file(filename);
  let mut contents = String::new();
  file.read_to_string(&mut contents).expect("Cannot read file");
  
  if contents != template{
    file.seek(SeekFrom::Start(0))?; // default is to write at the end of the file
    file.set_len(0)?; // remove all content which was there before
    file.write_all(template.as_bytes())?; // write new template into this file
    file.sync_all()?;
    Ok(true)
  }
  else{
    Ok(false)
  }
}
fn restart_nginx()  -> Result<(),std::io::Error>{
  let status = Command::new("/usr/sbin/service").arg("nginx").arg("reload").status()?;
  match status.code() {
    Some(_code) => {
      Ok(())
    }
    None       => {
      println!("Couldn't reload nginx.");
      exit(2);
    }
  }
}
fn get_file(filename: &str) -> std::fs::File{

	let file_handler = OpenOptions::new().read(true).write(true).create(true).open(filename);
	match file_handler{
    Ok(file) => file,
    Err(_) => {
      println!("Cannot create file '{}'. While it not existing.", filename);
      exit(3);
    },
  }
	
	
	
}

fn check_port(port: &str) -> bool{
  let port_re = Regex::new(r"^5\d{4}$").unwrap();
  let available = port_re.is_match(&port);
  return !available;  
}

fn check_port_exit(port: &str){
  let port_re = Regex::new(r"^5\d{4}$").unwrap();
  if !port_re.is_match(&port){
    println!("Port must be a number between 50 000 and 60 000. But was: {}", port);
    exit(5);
  }
  
}


fn check_domain(domain: &str){
  let domain_re = Regex::new(r"^[a-z0-9]{1,20}$").unwrap();
  // port is only in range of 50k to 60k
  

  if !domain_re.is_match(&domain){
    println!("Domain must be only lowercase letters and numbers up to 20. But it was: {}", domain);
    exit(4);
  }
}

fn create_template(sub_domain: &str, port: &str) -> String{
	// domain only lowercase letters or numbers, max 20
		check_port_exit(&port);
    check_domain(&sub_domain);
		


		format!(r#"server {{
    listen 80;
    server_name {sub_domain}.{domain_name};

    listen 443 ssl;
    ssl_certificate /etc/letsencrypt/live/{domain_name}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/{domain_name}/privkey.pem;

    access_log /var/log/nginx/{domain_name}.access.log;
    location / {{
      expires -1;

      proxy_redirect off;
      proxy_set_header   X-Real-IP            $remote_addr;
      proxy_set_header   X-Forwarded-For  $proxy_add_x_forwarded_for;
      proxy_set_header   X-Forwarded-Proto $scheme;
      proxy_set_header   Host                   $http_host;
      proxy_set_header   X-NginX-Proxy    true;
      proxy_set_header   Connection "";
      proxy_http_version 1.1;
      #proxy_cache one; # see link 1st line to add caching
      #proxy_cache_key sfs$request_uri$scheme;
      proxy_pass         http://127.0.0.1:{port};

      proxy_cache_bypass $http_upgrade;
      proxy_set_header Upgrade $http_upgrade;
      proxy_set_header Connection 'upgrade';
    }}
  }}"#, port=port, sub_domain=sub_domain, domain_name=env!("ZUNDER_DOMAIN"))
}



