use std::{
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Vec::from_iter(std::env::args());
    let tcp_port =
        args.get(1).ok_or("missing port number")?.parse::<u16>()?;
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, tcp_port))?;
    println!("tcp server waiting for connections on port '{}'", tcp_port);

    for incoming in listener.incoming() {
        let stream = incoming?;
        println!("new connection from {:?}", stream.peer_addr()?);
        std::thread::spawn(move || {
            if let Err(e) = handle_connection(stream) {
                eprintln!("ERROR: {}", e);
            }
        });
    }
    Ok(())
}

fn handle_connection(
    stream: TcpStream
) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = stream.try_clone()?;
    let mut input = BufReader::new(stream);
    loop {
        println!("\nwaiting for request from client...");
        let mut request = String::new();
        let r = input.read_line(&mut request)?;
        if r == 0 {
            println!("EOF");
            break;
        }

        println!("obtained {:?} from client", request);
        let reply = match request.trim().parse::<i32>() {
            Ok(value) => format!("{}\n", value * value),
            Err(e) => format!("invalid request: {}\n", e),
        };

        println!("sending reply {:?} to client...", reply);
        output.write_all(reply.as_bytes())?;
    }
    Ok(())
}