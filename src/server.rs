
//IMPORTS
use crate::message::EchoMessage;  //A protobuf-generated message type used for encoding and decoding data.
use log::{error, info, warn};     //log macros: error!, info!, warn! are used for logging.
use prost::Message;               //Used for encoding/decoding Protocol Buffers
use std::{
    io::{self, ErrorKind, Read, Write},      //Handles I/O (reading/writing to streams)
    net::{TcpListener, TcpStream},           //Provides networking utilities like TcpListener (server-side socket) and TcpStream (client-side connection).
    sync::{                              //Includes synchronization primitives
        atomic::{AtomicBool, Ordering, AtomicUsize},     //Manages a shared flag for server state, atomic types for managing client counts safely 
        Arc, Mutex,                             //Ensures thread-safe sharing of resources
    },
    thread,                       //Used for creating threads
    time::Duration,             // implementing delays.
};

//Client Struct
struct Client {               //Shared, thread-safe stream. The stream field holds the TCP connection to the client.
    stream: Arc<Mutex<TcpStream>>,
    retries: usize, // Track retry attempts for errors
}

//Client Implementation
impl Client {
    // 1- new() Method
    pub fn new(stream: TcpStream) -> Self {       
        Client {
            stream: Arc::new(Mutex::new(stream)),     //Constructs a new Client instance with the provided TcpStream
            retries: 0,
        }                         
    }
    
    // 2- handle() Method
    pub fn handle(&mut self) -> io::Result<()> {          
        let mut buffer = vec![0; 512];                         // 512-byte buffer to store incoming data, Reuse this buffer across read calls instead of re-allocating
        let mut stream = self.stream.lock().unwrap();     // Lock the stream
        // Read data from the client
        let bytes_read = stream.read(&mut buffer)?;        //Read client data
        if bytes_read == 0 {
            info!("Client disconnected.");
            return Ok(());
        }
//Message Handling: Decodes data into an EchoMessage, If successful, logs the content, serializes it back to bytes (encode_to_vec), and sends it to the client. Errors are logged if decoding fails
        match EchoMessage::decode(&buffer[..bytes_read]) { 
            Ok(message) => {
                info!("Received: {}", message.content);
                // Echo back the message
                let payload = message.encode_to_vec();                     //Serialize the response
                stream.write_all(&payload)?;        //Send it back
            }               
            Err(e) => {
                self.retries += 1;
                error!(
                    "Failed to decode message (attempt {}): {}", 
                    self.retries, e
                );
                if self.retries > 3 {
                    warn!("Too many decoding errors; disconnecting client.");
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Maximum retries reached",
                    ));
                }
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput, e.to_string()));   // Map DecodeError to a readable string
                
            }
        }

        Ok(())
    }
}

//Server Struct
pub struct Server {
    listener: TcpListener,                //Listens for incoming connections
    is_running: Arc<AtomicBool>,          // Shared running state, Ensures a shared, atomic flag to signal when the server is running.
    client_threads: Arc<Mutex<Vec<thread::JoinHandle<()>>>>, // Track active client threads
    client_count: Arc<AtomicUsize>, // Track the current number of clients connections using AtomicUsize.
    max_clients: usize,            // Maximum allowed clients connections
}

impl Server {
    // Creates a new server instance
    pub fn new(addr: &str, max_clients: usize) -> io::Result<Self> {      //new() Method : Initializes the server by binding it to the provided address and setting its initial state as stopped.
        let listener = TcpListener::bind(addr)?;                 // Bind to address
        let is_running = Arc::new(AtomicBool::new(false));        // Initialize running flag
        let client_threads = Arc::new(Mutex::new(Vec::new())); // Initialize client thread tracker
        let client_count = Arc::new(AtomicUsize::new(0));
        Ok(Server {
            listener,
            is_running,
            client_threads,
            client_count,
            max_clients,
        })
    }

    //run() Method
    // Runs the server, listening for incoming connections and handling them
    pub fn run(&self) -> io::Result<()> {
        self.is_running.store(true, Ordering::SeqCst);             // Set running flag
        // Set the listener to non-blocking mode
        self.listener.set_nonblocking(true)?;               //Make the listener non-blocking to avoid halting the program if there are no incoming connections.
        info!("Server is running on {}", self.listener.local_addr()?);  

       // Connection Handling Loop
        while self.is_running.load(Ordering::SeqCst) {
            match self.listener.accept() {
                Ok((mut stream, addr)) => {

                    let current_clients = self.client_count.load(Ordering::SeqCst);
                    if current_clients >= self.max_clients {
                        warn!("Connection refused: Max clients reached. Address: {}", addr);
                        
                        let _ = stream.write_all(b"Server is at full capacity.\n");

                        continue;
                    }

                    info!("New client connected: {}", addr);
                    self.client_count.fetch_add(1, Ordering::SeqCst);

                    let mut client = Client::new(stream);    // New client instance
                    // Handle each client in a separate thread
                    let is_running = self.is_running.clone();
                    let client_threads = self.client_threads.clone();
                    let client_count = self.client_count.clone();
                    let handle = thread::spawn(move || {
                        while is_running.load(Ordering::SeqCst) {
                            if let Err(e) = client.handle() {
                            error!("Error handling client ({}): {}", addr, e);
                                break;   // Disconnect on error
                            }   
                        }
                    // Decrement client count on disconnection
                    client_count.fetch_sub(1, Ordering::SeqCst);
                    info!("Client handler thread exiting for {}", addr);
                });
                client_threads.lock().unwrap().push(handle); // Track thread
            }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // No incoming connections, sleep briefly to reduce CPU usage
                    thread::sleep(Duration::from_millis(10));       // Tuned for quicker response
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);   // Log unexpected errors
                }
            }
        }
        self.cleanup_threads(); // Ensure proper cleanup on server stop
        info!("Server stopped.");
        Ok(())
    }

//stop() Method to Safely stops the server
    //Stops the server by setting the `is_running` flag to `false`
    pub fn stop(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            self.is_running.store(false, Ordering::SeqCst);  // Set running flag to false
            info!("Shutdown signal sent.");
        } else {
            warn!("Server was already stopped or not running.");
        }
    }
//ensures all threads complete execution before the server fully stops.
    fn cleanup_threads(&self) {
        let mut threads = self.client_threads.lock().unwrap();
        info!("Cleaning up {} client threads.", threads.len());
        for handle in threads.drain(..) {
            if let Err(e) = handle.join() {
                error!("Failed to join thread: {:?}", e);
            }
        }
    }
}
