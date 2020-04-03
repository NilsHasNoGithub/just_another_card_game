extern crate clap;
extern crate serde;
extern crate shared_lib;
extern crate think_thank_rust;

use clap::{App, Arg, ArgMatches};
use shared_lib::data_structures::{Card, GameState, GameStatePlayer, MenuState};
use shared_lib::socket_message_passing::{read_message, write_message};
use std::error::Error;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::TryRecvError;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

static VERSION: &str = "0.0";

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("Just another card game server")
        .version(VERSION)
        .author("Author: Nils Golembiewski")
        .about("Hosts a card game server to play card games on")
        .arg(
            Arg::with_name("port")
                .takes_value(true)
                .default_value("37012")
                .long("port")
                .help("The port to use as host"),
        )
        .get_matches();

    println!("Just another card game version {}", VERSION);

    main_loop(matches)?;

    Ok(())
}

struct ServerMessage {}

struct ClientMessage {}

#[derive(Debug)]
struct Client {
    id: String,
    pub sender_to_client: mpsc::Sender<GameStatePlayer>,
    pub receiver_from_client: mpsc::Receiver<String>,
}

impl Client {
    fn id(&self) -> &str {
        &self.id
    }
}

fn main_loop(matches: ArgMatches) -> Result<(), Box<dyn Error>> {
    let port = matches.value_of("port").unwrap();

    let clients = Arc::new(Mutex::new(Vec::new()));

    game_loop(clients.clone());
    server_loop(port, clients)?;
    Ok(())
}

fn server_loop(port: &str, clients: Arc<Mutex<Vec<Client>>>) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(&format!("127.0.0.1:{}", port))?;
    let executor = think_thank_rust::tasking::TaskPoolExecutor::new(16);

    for stream in listener.incoming() {
        let mut stream = stream?;
        let id = read_message(&mut stream)?;

        println!("{} has connected", id);

        let (sender_to_client, receiver_from_server) = mpsc::channel();
        let (sender_to_server, receiver_from_client) = mpsc::channel();

        clients.lock().unwrap().push(Client {
            id: id.clone(),
            sender_to_client,
            receiver_from_client,
        });

        executor.submit(move || 'a: loop {
            let mut br = false;
            let mut msg = read_message(&mut stream).unwrap_or_else(|_| {
                println!("{} has disconnected", id);
                br = true;
                "".into()
            });
            if br {
                break;
            }
            sender_to_server.send(msg).unwrap();
            thread::sleep(Duration::from_secs(1));
        })
    }
    Ok(())
}

fn game_loop(clients: Arc<Mutex<Vec<Client>>>) {
    thread::spawn(move || {
        let mut game_state = GameState::Menu(MenuState::new(Vec::new()));
        loop {
            {
                let mut clients = clients.lock().unwrap();
                let mut to_remove = Vec::new();
                for (i, client) in clients.iter().enumerate() {
                    let client_msg_ = client.receiver_from_client.try_recv();
                    let mut client_msg = None;
                    match client_msg_ {
                        Ok(s) => client_msg = Some(s),

                        Err(e) => {
                            if e == TryRecvError::Disconnected {
                                to_remove.push(i);
                            }
                        }
                    };

                    println!("Client: {}, says: {:?}", client.id(), client_msg)
                }
                to_remove.sort();
                to_remove.reverse();
                for i in to_remove {
                    println!("Removing client: {:?}", clients[i]);
                    clients.remove(i);
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    });
}

fn cli_loop() {
    todo!();
}
