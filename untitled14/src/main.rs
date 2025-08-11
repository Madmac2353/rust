use std::{
    io::{self, BufRead, Read, Write},
    net::TcpStream,
    thread,
};

fn main() -> io::Result<()> {
    let mut stream = TcpStream::connect("56269d8e-5a0f-4237-acef-ad0406d44cb3-00-v22yvcl26s2s.janeway.replit.dev:80")?;
    println!("Connected to chat server!");
    println!("Type messages and press Enter to send");
    println!("Press Ctrl+C to exit\n");

    let mut reader_stream = stream.try_clone()?;

    // Spawn thread to handle incoming messages
    thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match reader_stream.read(&mut buffer) {
                Ok(0) => {
                    println!("\nServer disconnected");
                    break;
                }
                Ok(n) => {
                    let message = String::from_utf8_lossy(&buffer[..n]);
                    print!("\r{}> ", message); // Clear line and show prompt
                    io::stdout().flush().unwrap();
                }
                Err(e) => {
                    eprintln!("\nRead error: {}", e);
                    break;
                }
            }
        }
        std::process::exit(0);
    });

    // Main thread handles user input
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut input_buffer = String::new();

    loop {
        print!("> ");
        io::stdout().flush()?;
        input_buffer.clear();

        match stdin_lock.read_line(&mut input_buffer) {
            Ok(0) => break, // EOF (Ctrl+D)
            Ok(_) => {
                let input = input_buffer.trim();
                if !input.is_empty() {
                    if let Err(e) = stream.write_all(input.as_bytes()) {
                        eprintln!("Send error: {}", e);
                        break;
                    }
                    if let Err(e) = stream.write_all(b"\n") {
                        eprintln!("Send error: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        }
    }

    Ok(())
}