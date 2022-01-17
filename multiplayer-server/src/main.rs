use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::io::Write;
use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};

#[derive(Debug, Clone, Copy)]
struct Vector3 {
    x: f32,
    y: f32,
    z: f32
}

#[derive(Debug, Clone, Copy)]
enum MessageType {
    End,
    Connect,
    Position(Vector3),
    Invalid,
}

impl MessageType {
    fn serialize(&self, stream: &mut impl Write) {
        match self {
            MessageType::End => {
                stream.write_i16::<LittleEndian>(0).unwrap();
            },
            MessageType::Connect => {
                stream.write_i16::<LittleEndian>(1).unwrap();
            },
            MessageType::Position(pos) => {
                stream.write_i16::<LittleEndian>(2).unwrap();
                stream.write_f32::<LittleEndian>(pos.x).unwrap();
                stream.write_f32::<LittleEndian>(pos.y).unwrap();
                stream.write_f32::<LittleEndian>(pos.z).unwrap();
            }
            MessageType::Invalid => (),
        }
    }
    
    fn deserialize(stream: &mut TcpStream) -> MessageType {
        match stream.read_i16::<LittleEndian>().unwrap() {
            0 => MessageType::End,
            1 => MessageType::Connect,
            2 => MessageType::Position(Vector3 { x: stream.read_f32::<LittleEndian>().unwrap(), y: stream.read_f32::<LittleEndian>().unwrap(), z: stream.read_f32::<LittleEndian>().unwrap() }),
            t => { eprintln!("Invalid message type: {}", t); MessageType::Invalid }
        }
    }
}

struct PlayerConnection {
    id: i32,
    sender: Sender<(i32, MessageType)>
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:1245").unwrap();

    let mut next_player_id = 1;
    let connections = Arc::new(Mutex::new(Vec::new()));

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        let (tx, rx) = mpsc::channel();
        let player_id = next_player_id;
        next_player_id += 1;

        let connection = PlayerConnection { id: player_id, sender: tx };
        let connections = Arc::clone(&connections);
        connections.lock().unwrap().push(connection);

        thread::spawn(move || {
            let connections = Arc::clone(&connections);
            handle_connection(stream, player_id, rx, connections);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream, player_id: i32, receiver: Receiver<(i32, MessageType)>, connections: Arc<Mutex<Vec<PlayerConnection>>>) {
    println!("Connected!!!!");

    for connection in &*Arc::clone(&connections).lock().unwrap() {
        if connection.id != player_id {
            connection.sender.send((connection.id, MessageType::Connect)).unwrap();
        }
    }

    let mut buffer: Vec<u8> = Vec::new();
    
    loop {
        let msg = MessageType::deserialize(&mut stream);
        println!("Message from player {}: {:?}", player_id, msg);
        
        for connection in &*Arc::clone(&connections).lock().unwrap() {
            if connection.id != player_id {
                connection.sender.send((player_id, msg.clone())).unwrap();
            }
        }

        buffer.clear();
        while let Ok(msg) = receiver.try_recv() {
            let (sender, msg) = msg;
            buffer.write_i32::<LittleEndian>(sender).unwrap();
            msg.serialize(&mut buffer);
        }
        buffer.write_i32::<LittleEndian>(0).unwrap();
        MessageType::End.serialize(&mut buffer);
        stream.write(&buffer).unwrap();
    }
}
