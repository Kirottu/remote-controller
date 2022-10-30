use clap::Parser;
use std::{
    collections::HashMap,
    fs,
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    thread,
};

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    config: PathBuf,
}

#[derive(Debug)]
enum Method {
    Get,
    Post,
}

#[derive(Debug)]
struct Request {
    method: Method,
    path: String,
    headers: HashMap<String, String>,
}

#[derive(Debug)]
enum RequestError {
    InvalidMethod,
}

#[derive(serde::Deserialize)]
struct Config {
    address: String,
    wol_config: WolConfig,
}

impl Config {
    pub fn load(path: &PathBuf) -> io::Result<Self> {
        let string = fs::read_to_string(path)?;
        match ron::from_str::<Config>(&string) {
            Ok(config) => Ok(config),
            Err(why) => Err(io::Error::new(io::ErrorKind::Other, why)),
        }
    }
}

#[derive(serde::Deserialize)]
struct WolConfig {
    mac_address: String,
}

impl Request {
    pub fn from_stream(stream: &mut TcpStream) -> Result<Self, RequestError> {
        let reader = BufReader::new(stream);
        let lines = reader
            .lines()
            .map(|line| line.unwrap())
            .take_while(|line| !line.is_empty())
            .collect::<Vec<String>>();

        let request_split = lines[0].split(" ").collect::<Vec<&str>>();
        let method = match request_split[0] {
            "GET" => Method::Get,
            "POST" => Method::Post,
            _ => return Err(RequestError::InvalidMethod),
        };
        let path = request_split[1].to_string();

        let headers = lines
            .iter()
            .skip(1)
            .map(|line| line.split_once(": ").unwrap())
            .map(|(name, val)| (name.to_string(), val.to_string()))
            .collect::<HashMap<String, String>>();

        Ok(Request {
            method,
            path,
            headers,
        })
    }
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let config = Box::leak(Box::new(Config::load(&args.config)?));

    let listener = TcpListener::bind(&config.address)?;

    for stream in listener.incoming() {
        thread::spawn(|| match handle_request(stream, config) {
            Ok(_) => (),
            Err(why) => {
                println!("Error handling request: {}", why);
            }
        });
    }

    Ok(())
}

fn handle_request(stream: std::io::Result<TcpStream>, config: &Config) -> std::io::Result<()> {
    let mut stream = stream?;
    let request = match Request::from_stream(&mut stream) {
        Ok(request) => request,
        Err(why) => {
            stream.write_all(b"HTTP/1.1 418 I'm a teapot\r\n\r\n")?;
            return Err(io::Error::new(io::ErrorKind::Other, format!("{:?}", why)));
        }
    };
    println!("{:?}", request);
    match request.method {
        Method::Get => match request.path.as_str() {
            "/ping" => stream.write_all(b"HTTP/1.1 200 OK\r\n\r\npong")?,
            _ => stream.write_all(b"HTTP/1.1 404 Not found\r\n\r\n")?,
        },
        Method::Post => match request.path.as_str() {
            "/turn_on" => {
                let wol = wakey::WolPacket::from_string(&config.wol_config.mac_address, ':');
                match wol.send_magic() {
                    Ok(_) => stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n")?,
                    Err(why) => stream.write_all(
                        format!("HTTP/1.1 500 Internal Server Error\r\n\r\n{}", why).as_bytes(),
                    )?,
                }
            }
            _ => stream.write_all(b"HTTP/1.1 404 Not found\r\n\r\n")?,
        },
    }

    Ok(())
}
