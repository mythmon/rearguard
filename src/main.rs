#![feature(collections)]
#![feature(plugin)]
#![feature(std_misc)]

use std::io::{BufReader, BufRead, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::mpsc;

mod message;

use message::IrcMessage;


fn main() {
    let listener = TcpListener::bind("127.0.0.1:4567").unwrap_or_else(|e| { panic!(e) });
    println!("Listening {}", listener.local_addr().unwrap_or_else(|e| { panic!(e) }));

    let (broadcast_tx, broadcast_rx) = mpsc::channel::<IrcMessage>();
    let (new_clients_tx, new_clients_rx) = mpsc::channel::<TcpStream>();
    let mut client_txs = vec![];

    thread::Builder::new().name("gather_clients".to_string())
        .spawn(move || { gather_clients(listener, new_clients_tx); })
        .unwrap_or_else(|e| { panic!(e) });

    loop {
        select! (
            new_client = new_clients_rx.recv() => {
                let new_client = new_client.unwrap_or_else(|e| { panic!(e) });
                let broadcast_tx_clone = broadcast_tx.clone();
                let (client_tx, client_rx) = mpsc::channel();
                client_txs.push(client_tx);

                thread::Builder::new().name(format!("handle_client ({:?})", new_client))
                    .spawn(move || {
                        handle_client(new_client, broadcast_tx_clone, client_rx)
                            .unwrap_or_else(|e| { panic!(e) });
                    })
                    .unwrap_or_else(|e| { panic!(e) });

            },

            msg = broadcast_rx.recv() => {
                for client_tx in &client_txs[..] {
                    let msg = msg.clone().unwrap_or_else(|e| { panic!(e) });
                    client_tx.send(msg).unwrap_or_else(|e| { panic!(e) });
                }
            }
        );
    }
}


fn gather_clients(listener: TcpListener, chan: mpsc::Sender<TcpStream>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                chan.send(stream).unwrap_or_else(|e| { panic!(e) });
            },
            Err(e) => {
                println!("Error with incoming stream: {}", e);
            },
        }
    }
}


fn handle_client(
    mut stream: TcpStream,
    broadcast_tx: mpsc::Sender<IrcMessage>,
    broadcast_rx: mpsc::Receiver<IrcMessage>
) -> Result<()> {
    let client_addr = try!(stream.peer_addr());
    let reader = BufReader::new(try!(stream.try_clone()));

    println!("{}: connected.", client_addr);

    let (messages_tx, messages_rx) = mpsc::channel::<IrcMessage>();
    thread::Builder::new().name(format!("gather_client_messages ({})", client_addr))
        .spawn(move || { gather_client_messages(reader, messages_tx); })
        .unwrap_or_else(|e| { panic!(e) });

    let mut my_nick: Option<String>;
    let host_prefix = Some("rearguard.local");
    let mut sent_welcome = false;

    loop {
        select! (
            user_msg = messages_rx.recv() => {
                let user_msg = user_msg.unwrap_or_else(|e| { panic!(e) });
                println!("{}: {}", client_addr, user_msg.to_string());

                match &user_msg.command[..] {
                    "NICK" => {
                        my_nick = Some(user_msg.params[0].clone());
                        let nick_for_msg = &my_nick.expect("wat")[..];
                        if !sent_welcome {
                            let reply_msg = IrcMessage::new(host_prefix, "001", vec![nick_for_msg], Some("Welcome"));
                            write_irc_message(&mut stream, reply_msg);
                            sent_welcome = true;
                        }
                    },
                    "QUIT" => {
                        let reply_msg = IrcMessage::new(host_prefix, "QUIT", vec![], Some("Client Quit"));
                        write_irc_message(&mut stream, reply_msg);
                        break;
                    },
                    "PING" => {
                        let reply_msg = IrcMessage::new(host_prefix, "PONG", vec![host_prefix.expect("wat")], Some(&user_msg.params[0][..]));
                        write_irc_message(&mut stream, reply_msg);
                    },
                    "USER" => {},
                    _ => {
                        println!("Unknown message! {}", user_msg.to_string());
                    },
                }
            },

            broadcast_msg = broadcast_rx.recv() => {
                let broadcast_msg = broadcast_msg.unwrap_or_else(|e| { panic!(e) });
                try!(write!(stream, "broadcast: {:?}", broadcast_msg));
            }
        );
    }

    println!("{}: disconnected.", client_addr);
    Ok(())
}


fn gather_client_messages(reader: BufReader<TcpStream>, chan: mpsc::Sender<IrcMessage>) {
    for line in reader.lines() {
        let line = line.unwrap_or_else(|e| { panic!(e) });
        match line.parse::<IrcMessage>() {
            Ok(msg) => chan.send(msg).unwrap_or_else(|e| { println!("{}", e); panic!(e) }),
            Err(_) => {},
        };
    }
}

fn write_irc_message<T: Write>(target: &mut T, msg: IrcMessage) {
    let msg_string = msg.to_string();
    println!("Sending: {}", msg_string);
    write!(*target, "{}\r\n", msg_string).unwrap();
}
