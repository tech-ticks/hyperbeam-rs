#![feature(proc_macro_hygiene)]
#![feature(asm)]

use hyperbeam_rtdx::modpack::{ModpackMetadata, MODPACK_BASE_PATH};
use hyperbeam_unity::{reflect, texture_helpers, IlString};
use lazy_static;
use pmdrtdx_bindings::*;
use skyline::nn;
use skyline::{hook, install_hook, install_hooks};
use std::ffi::{CString, c_void};
use std::os::raw::c_char;
use std::ptr::{self, null_mut};
use std::string::String;
use std::slice;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt, ReadBytesExt};
use std::io::{Write, Cursor, Read};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Mutex;
use std::error::Error;
use std::env;

struct Player {
    id: i32,
    position: Vector3,
    model: usize, // Actually a pointer, this is terrible
}

lazy_static::lazy_static! {
    static ref CLIENT: Client = Client::new();
    static ref PLAYERS: Mutex<Vec<Player>> = Mutex::new(Vec::new());
}

// GroundMode_ groundMode, GroundPlayContext context, bool bResetCamHero = true
#[hook(replace = GroundPlayer_SetGroundMode_)]
unsafe fn hook_ground_player_set_ground_mode(this_ptr: *mut GroundPlayer, ground_mode: i32, context: *mut GroundPlayer_GroundPlayContext, reset_cam_hero: bool) -> i32 {
    let player = GroundPlayer_get_actCh_Player_(this_ptr, null_mut());
    if !player.is_null() && ground_mode == 1 {
        let pivot = ActObjCharactor_get_worldPivot(player, null_mut());
        // println!("GroundPlayer SetGroundMode: {} @{}, {}, {}", ground_mode, pivot.x, pivot.y, pivot.z);
        CLIENT.send_position(pivot);

        while let Ok((sender, message)) = CLIENT.get_message() {
            match message {
                MessageType::Connect => {
                    println!("Player {} connected.", sender);
                    let act_object_manager = SingletonMonoBehaviour_1_ActObjectManager__get_Instance(SingletonMonoBehaviour_1_ActObjectManager__get_Instance__MethodInfo);
                    let model = ActObjectManager_GetActObj_5(act_object_manager, IlString::new("PARTNER").as_ptr(), ActObjectManager_GetActObj_2__MethodInfo) as *mut ActObjCharactor;
                    PLAYERS.lock().unwrap().push(Player {
                       id: sender, position: Vector3 { x: 0.0, y: 0.0, z: 0.0 }, model: model as _
                    });
                }
                MessageType::Position(vec) => {
                    println!("Got position of player {}: {} {} {}", sender, vec.x, vec.y, vec.z);
                    let act_object_manager = SingletonMonoBehaviour_1_ActObjectManager__get_Instance(SingletonMonoBehaviour_1_ActObjectManager__get_Instance__MethodInfo);
                    let model = ActObjectManager_GetActObj_5(act_object_manager, IlString::new("PARTNER").as_ptr(), ActObjectManager_GetActObj_2__MethodInfo) as *mut ActObjCharactor;
                    if !model.is_null() {
                        //if let Some(character) = PLAYERS.lock().unwrap().iter().find(|p| p.id == sender) {
                        //let model = character.model as *mut ActObjCharactor;
                        ActObjCharactor_set_worldPivot(model, vec, null_mut());
                        //}
                    }
                },
                MessageType::End => {
                    println!("Got end message.");
                }
                _ => {}
            }
        }
    }

    call_original!(this_ptr, ground_mode, context, reset_cam_hero)
}

#[skyline::main(name = "multiplayer_test")]
pub fn main() {
    install_hooks!(hook_ground_player_set_ground_mode);
}

#[derive(Clone, Copy)]
enum MessageType {
    End,
    Connect,
    Position(Vector3),
    Invalid
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
            },
            MessageType::Invalid => (),
        }
    }

    fn deserialize(stream: &mut Cursor<Vec<u8>>) -> MessageType {
        match stream.read_i16::<LittleEndian>().unwrap() {
            0 => MessageType::End,
            1 => MessageType::Connect,
            2 => MessageType::Position(Vector3 { x: stream.read_f32::<LittleEndian>().unwrap(), y: stream.read_f32::<LittleEndian>().unwrap(), z: stream.read_f32::<LittleEndian>().unwrap() }),
            t => { eprintln!("Invalid message type: {}", t); MessageType::Invalid }
        }
    }
}

struct Client {
    sender: Mutex<Sender<MessageType>>,
    receiver: Mutex<Receiver<(i32, MessageType)>>,
}

impl Client {
    fn new() -> Client {
        let (tx_main, rx_worker) = mpsc::channel();
        let (tx_worker, rx_main) = mpsc::channel();

        thread::spawn(move || {
            let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 188, 20)), 1245) ;// "192.168.188.20:1245".parse().unwrap();
            let mut stream = TcpStream::connect(&addr).unwrap();
            println!("Connected.");

            let mut buffer: Vec<u8> = Vec::new();

            MessageType::Connect.serialize(&mut buffer);
            stream.write(&buffer).unwrap();

            while let Ok(msg) = rx_worker.recv() {
                let msg: MessageType = msg;
                buffer.clear();
                msg.serialize(&mut stream);
                stream.write(&buffer).unwrap();

                let mut buffer: Vec<u8> = vec![0; 1024];
                if let Ok(len) = stream.read(&mut buffer) {
                    let mut cursor = Cursor::new(buffer);

                    loop {
                        let sender = cursor.read_i32::<LittleEndian>().unwrap();
                        let message = MessageType::deserialize(&mut cursor);
                        match message {
                            MessageType::End => {
                                break;
                            }
                            _ => {}
                        }
                        tx_worker.send((sender, message)).unwrap();
                    }
                }
            }
        });

        Client { sender: Mutex::new(tx_main), receiver: Mutex::new(rx_main) }
    }

    fn send_position(&self, pos: Vector3) {
        self.sender.lock().unwrap().send(MessageType::Position(pos)).unwrap();
    }

    fn get_message(&self) -> Result<(i32, MessageType), TryRecvError> {
        self.receiver.lock().unwrap().try_recv()
    }
}

