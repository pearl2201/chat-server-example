// An attribute to hide warnings for unused code.
#![allow(dead_code)]

use bufstream::BufStream;
use std::io;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

#[derive(Debug, Clone)]
enum TypeBroadcastMessage {
    ALL,
    EXCEPT,
    ONLY,
}

#[derive(Debug, Clone)]
struct MessageChannel {
    message: String,
    sender: SocketAddr,
    message_type: TypeBroadcastMessage,
    except_ids: Option<Vec<SocketAddr>>,
    include_ids: Option<Vec<SocketAddr>>,
}

fn handle_listen_broadcast(
    mut stream: TcpStream,
    receiver: mpsc::Receiver<MessageChannel>,
    name: std::net::SocketAddr,
) {

    loop {
        let message = receiver.recv().unwrap();
        if message.message == "close" && message.sender == name {
            break;
        }

        let text_content = format!("message: {:?}", message);
        let text_content = text_content.as_bytes();
        let except_ids = message.except_ids;
        let include_ids = message.include_ids;

        let mut is_send = false;
        match message.message_type {
            TypeBroadcastMessage::ALL => {
                is_send = true;
            }
            TypeBroadcastMessage::EXCEPT => {
                if let Some(except_ids) = except_ids {
                    match except_ids.into_iter().find(|addr| addr == &name) {
                        Some(_) => is_send = false,
                        None => is_send = true,
                    }
                }
            }
            TypeBroadcastMessage::ONLY => {
                if let Some(include_ids) = include_ids {
                    match include_ids.into_iter().find(|addr| addr == &name) {
                        Some(_) => is_send = true,
                        None => is_send = false,
                    }
                }
            }
        }
        if is_send {
            stream.write_all(text_content).unwrap();
        }

    }
}

fn handle_client(
    stream: TcpStream,
    receiver: mpsc::Receiver<MessageChannel>,
    sender: Arc<Mutex<mpsc::Sender<MessageChannel>>>,
) {
    stream.set_nodelay(true).expect("set_nodelay call failed");
    let addr = stream.peer_addr().unwrap();
    let stream_clone = stream.try_clone().expect("clone failed...");
    stream_clone.set_nodelay(true).expect("set_nodelay on clone call failed");
    let mut buf_stream = BufStream::new(stream);

    
    thread::spawn(move || handle_listen_broadcast(stream_clone, receiver, addr));
    loop {

        let mut buffer = String::new();
        let byte_reads = buf_stream.read_line(&mut buffer).unwrap();
        if byte_reads != 0 {
            println!(" Server receive from {}: {}", addr, buffer);

            let message = MessageChannel {
                message: buffer.clone(),
                include_ids: None,
                except_ids: None,
                sender: addr,
                message_type: TypeBroadcastMessage::ALL,

            };
            sender.lock().unwrap().send(message).unwrap();
            if buffer == "close" {
                break;
            }
        }

    }

}

fn handle_server(
    receiver: mpsc::Receiver<MessageChannel>,
    senders: Arc<Mutex<Vec<mpsc::Sender<MessageChannel>>>>,
) {
    loop {
        let message = receiver.recv().unwrap();
        let temp_senders = senders.lock().unwrap();
        for sender in temp_senders.iter() {
            sender.send(message.clone()).unwrap();
        }
    }
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:5000")?;


    let (server_sender, server_receiver) = channel();
    let server_sender = Arc::new(Mutex::new(server_sender));

    let vec_senders: Vec<mpsc::Sender<MessageChannel>> = Vec::new();

    let vec_senders = Arc::new(Mutex::new(vec_senders));
    {
        let temp_vec_sender = Arc::clone(&vec_senders);
        thread::spawn(move || handle_server(server_receiver, temp_vec_sender));
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("new connection: {}", stream.peer_addr().unwrap());

                let (process_sender, process_receiver) = channel();

                let temp_server_sender = Arc::clone(&server_sender);
                let temp_vec_sender = Arc::clone(&vec_senders);
                temp_vec_sender.lock().unwrap().push(process_sender);
                thread::spawn(move || handle_client(stream, process_receiver, temp_server_sender));
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }

    }

    Ok(())
}