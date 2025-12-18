use std::{
    io::{BufRead, BufReader, Write}, net::{Ipv4Addr, TcpListener, TcpStream}
};
use serde::{Deserialize, Serialize};
use std::sync::{Mutex};
use std::sync::atomic::{AtomicUsize};
use std::sync::Arc;


#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Image{
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug)]
struct Client{
    output:TcpStream,
    id:usize,
    image:Image,
    position:Point,
}

#[derive(Debug, Serialize, Deserialize)]
struct Player{
    id: usize,
    position: Point,
    image: Image,
}

#[derive(Debug, Default)]
struct Server{
    next_n: AtomicUsize,
    clients:Mutex<Vec<Client>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Vec::from_iter(std::env::args());
    let tcp_port =
        args.get(1).ok_or("missing port number")?.parse::<u16>()?;
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, tcp_port))?;
    println!("tcp server waiting for connections on port '{}'", tcp_port);

    let server = Arc::new(Server::default());
    for incoming in listener.incoming() {
        let stream = incoming?;
        let server = Arc::clone(&server);
        println!("new connection from {:?}", stream.peer_addr()?);
        std::thread::spawn(move || {
            if let Err(e) = handle_connection(stream, server.as_ref()) {
                eprintln!("ERROR: {}", e);
            }
        });
    }
    Ok(())
}

fn handle_connection(
    stream: TcpStream,
    server: &Server,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = stream.try_clone()?;
    let mut input = BufReader::new(stream);
    loop {
        println!("\nwaiting for request from client...");
        let mut request = String::new();
        let r = input.read_line(&mut request)?;
        if r == 0 {
            println!("EOF");
            let mut clients = server.clients.lock().unwrap();
            
            if let Some(pos) = clients.iter().position(|c| c.output.peer_addr().unwrap() == output.peer_addr().unwrap()) {

                let disconnected_id = clients[pos].id;
    
                for c in clients.iter() {
                    match c.output.try_clone() {
                        Ok(mut stream) => {
                            stream.write_all(format!("disconnect {}\n", disconnected_id).as_bytes()).ok();
                },
                Err(e) => eprintln!("Failed to clone stream for client {}: {}", c.id, e),
        }
    }

    clients.retain(|c| c.output.peer_addr().unwrap() != output.peer_addr().unwrap());
}
            break;
        }

        println!("obtained {:?} from client", request);

        let reply= match request.split_once(' ') 
        {
            Some(("motion", data)) => {
                let (id, pos) = serde_json::from_str::<(usize, Point)>(data.trim())?;

                {
                    let mut clients = server.clients.lock().unwrap();
                    for c in clients.iter_mut() {
                        if c.id == id {
                            c.position.x += pos.x;
                            c.position.y += pos.y;
                        }else {
                            match c.output.try_clone() {
                                Ok(mut stream) => {
                                    stream.write_all(format!("position {}\n",serde_json::to_string(&(&id, &pos))?).as_bytes())?;

                                },
                                Err(e) => {
                                    eprintln!("Failed to clone stream for client {}: {}", c.id, e);
                                }
                            }
                        }
                    }
                    
                }
                format!("position {}\n", serde_json::to_string(&(id, pos))?)
            }
            Some(("newplayer", data)) => {
                let image = serde_json::from_str::<Image>(data.trim())?;
                println!("received newplayer: ");

                

                let id = server.next_n.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let position = Point{x:0,y:0};

                let client = Client{
                    output: output.try_clone()?,
                    id,
                    image,
                    position,
                };

                let mut clients = server.clients.lock().unwrap();
                
                let existing_players: Vec<Player> = clients.iter().map(|c| Player {
                    id: c.id,
                    position: c.position,
                    image: c.image.clone(),
                }).collect();

                for c in clients.iter() {
                    println!("client id: {}, position: {:?}", c.id, c.position);
                    match c.output.try_clone() {
                        Ok(mut stream) => {
                            stream.write_all(format!("newplayerconnect {}\n", serde_json::to_string(&(&client.id, &client.position, &client.image))?).as_bytes())?;
                        },
                        Err(e) => {
                            eprintln!("Failed to clone stream for client {}: {}", c.id, e);
                        }
                    }
                }

                clients.push(client); 

                format!("added {}\n", serde_json::to_string(&(&id, 
                                                            &position,
                                                            &existing_players))?)
            }
            _ => match request.trim().parse::<i32>() {
                Ok(_) => format!("Unknown received"),
                Err(e) => format!("invalid request: {}\n", e),
            },
        };
        println!("sending reply {:?} to client...", reply);
        output.write_all(reply.as_bytes())?;
       
    }  
    Ok(())
}