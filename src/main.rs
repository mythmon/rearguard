#![feature(collections)]
#![feature(plugin)]
#![feature(std_misc)]

use std::io::{BufReader, BufRead, Result, Write};
use std::net::{self, TcpListener, TcpStream};
use std::thread;
use std::sync::mpsc;

mod message;


/// An irc client
struct Client {
    id: u32,
    channel: mpsc::Sender<message::IrcMessage>,
}


fn main() {
    let listener = TcpListener::bind("127.0.0.1:4567").unwrap();
    println!("Listening {}", listener.local_addr().unwrap());

    let (broadcast_tx, broadcast_rx) = mpsc::channel::<u8>();
    let (new_clients_tx, new_clients_rx) = mpsc::channel::<TcpStream>();
    let mut client_txs = vec![];

    thread::spawn(move || { gather_clients(listener, new_clients_tx); });

    loop {
        select! (
            new_client = new_clients_rx.recv() => {
                let new_client = new_client.unwrap();
                let broadcast_tx_clone = broadcast_tx.clone();
                let (client_tx, client_rx) = mpsc::channel();
                client_txs.push(client_tx);

                thread::spawn(move || {
                    handle_client(new_client, broadcast_tx_clone, client_rx).unwrap();
                });
            },

            msg = broadcast_rx.recv() => {
                let msg = msg.unwrap();
                for client_tx in &client_txs[..] {
                    client_tx.send(msg).unwrap();
                }
            }
        );
    }
}


fn gather_clients(listener: TcpListener, chan: mpsc::Sender<TcpStream>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                chan.send(stream).unwrap();
            },
            Err(e) => {
                println!("Error with incoming stream: {}", e);
            },
        }
    }
}


fn handle_client(mut stream: TcpStream, broadcast_tx: mpsc::Sender<u8>, broadcast_rx: mpsc::Receiver<u8>) -> Result<()> {
    let client_addr = try!(stream.peer_addr());
    let reader = BufReader::new(try!(stream.try_clone()));

    println!("{}: connected.", client_addr);

    let (messages_tx, messages_rx) = mpsc::channel::<String>();
    thread::spawn(move || { gather_client_messages(reader, messages_tx); });

    loop {
        select! (
            user_msg = messages_rx.recv() => {
                let user_msg = user_msg.unwrap();
                println!("{}: {}", client_addr, user_msg);

                match &user_msg[..] {
                    "ping" => {
                        try!(write!(stream, "pong\n"));
                    },
                    "exit" => {
                        try!(write!(stream, "goodbye\n"));
                        try!(stream.shutdown(net::Shutdown::Both));
                        break
                    },
                    "msg" => {
                        broadcast_tx.send(42).unwrap();
                        try!(write!(stream, "ok\n"));
                    },
                    _ => {
                        try!(write!(stream, "unknown\n"));
                    }
                }
            },

            broadcast_msg = broadcast_rx.recv() => {
                let broadcast_msg = broadcast_msg.unwrap();
                try!(write!(stream, "broadcast: {}", broadcast_msg));
            }
        );
    }

    println!("{}: disconnected.", client_addr);
    Ok(())
}


fn gather_client_messages(reader: BufReader<TcpStream>, chan: mpsc::Sender<String>) {
    for line in reader.lines() {
        let line = line.unwrap();
        chan.send(line).unwrap();
    }
}
