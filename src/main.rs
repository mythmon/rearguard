use std::io::{BufReader, BufRead, Result, Write};
use std::net::{self, TcpListener, TcpStream};
use std::thread;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4567").unwrap();

    let port = listener.local_addr().unwrap().port();

    println!("Listening on 127.0.0.1:{}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream)
                });
            },
            Err(e) => println!("Error: {}", e),
        }
    }

    drop(listener);
}

fn handle_client(mut stream: TcpStream) -> Result<()> {
    let client_addr = try!(stream.peer_addr());
    let reader = BufReader::new(try!(stream.try_clone()));

    println!("{}: connected.", client_addr);

    for maybe_line in reader.lines() {
        let line = try!(maybe_line);
        println!("{}: {}", client_addr, line);

        match &line[..] {
            "ping" => {
                try!(write!(stream, "pong\n"));
            },
            "exit" => {
                try!(write!(stream, "goodbye\n"));
                try!(stream.shutdown(net::Shutdown::Both));
                break
            },
            _ => {
                try!(write!(stream, "unknown\n"));
            }
        }
    }

    println!("{}: disconnected.", client_addr);

    Ok(())
}
