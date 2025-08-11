use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Arc, Mutex},
    thread,
};

type Sender = mpsc::Sender<String>;
type ClientMap = Arc<Mutex<HashMap<usize, Sender>>>;

fn handle_client(
    stream: TcpStream,
    id: usize,
    clients: ClientMap,
    broadcast_tx: mpsc::Sender<(usize, String)>,
) {
    let addr = stream.peer_addr().unwrap();
    println!("Client {} connected from {}", id, addr);

    let mut reader = stream.try_clone().expect("Failed to clone stream");
    let mut writer = stream;

    // Send welcome message
    let welcome_msg = format!("Welcome! Your ID: {}\n", id);
    if writer.write_all(welcome_msg.as_bytes()).is_err() {
        println!("Client {}: Send welcome failed", id);
        return;
    }

    // Create client channel
    let (client_tx, client_rx) = mpsc::channel();

    // Add to client map
    {
        let mut clients = clients.lock().unwrap();
        clients.insert(id, client_tx);
    }

    broadcast_tx.send((id, format!("User {} joined!\n", id))).unwrap();

    // Writer thread
    let writer_thread = thread::spawn(move || {
        for msg in client_rx {
            if writer.write_all(msg.as_bytes()).is_err() {
                break;
            }
        }
        println!("Client {} writer stopped", id);
    });

    // Read messages
    let mut buffer = [0; 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break, // Connection closed by client
            Ok(n) => {
                let msg = match String::from_utf8(buffer[..n].to_vec()) {
                    Ok(msg) => msg,
                    Err(e) => {
                        eprintln!("Client {}: Invalid UTF-8 sequence: {}", id, e);
                        continue;
                    }
                };

                // Trim whitespace and ignore empty messages
                let msg = msg.trim();
                if !msg.is_empty() {
                    if let Err(e) = broadcast_tx.send((id, format!("[User {}]: {}\n", id, msg))) {
                        eprintln!("Client {}: Failed to broadcast message: {}", id, e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Client {}: Read error: {}", id, e);
                break;
            }
        }
    }

    // Cleanup
    {
        let mut clients = clients.lock().unwrap();
        clients.remove(&id);
    }
    broadcast_tx.send((id, format!("User {} left!\n", id))).unwrap();
    writer_thread.join().unwrap(); // Wait for writer thread

    println!("Client {} disconnected", id);
}

// ... rest of the server code remains the same ...
fn broadcast_handler(clients: ClientMap, rx: mpsc::Receiver<(usize, String)>) {
    for (sender_id, msg) in rx {
        let clients = clients.lock().unwrap();
        for (&id, tx) in clients.iter() {
            if id != sender_id {
                if tx.send(msg.clone()).is_err() {
                    println!("Failed to send to client {}", id);
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:24115")?;
    println!("Chat server listening on port 24115");

    let clients: ClientMap = Arc::new(Mutex::new(HashMap::new()));
    let (broadcast_tx, broadcast_rx) = mpsc::channel();
    let clients_clone = Arc::clone(&clients);

    // Start broadcast handler thread
    thread::spawn(move || broadcast_handler(clients_clone, broadcast_rx));

    let mut next_id = 0;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clients = Arc::clone(&clients);
                let broadcast_tx = broadcast_tx.clone();

                thread::spawn(move || {
                    handle_client(stream, next_id, clients, broadcast_tx);
                });

                next_id += 1;
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
    Ok(())
}