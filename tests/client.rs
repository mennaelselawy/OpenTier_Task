
//This code sets up a TCP client that can connect to a server, send and receive messages, and handle disconnections.

//IMPORTS
use embedded_recruitment_task::message::EchoMessage;      // embedded_recruitment_task Crate
use log::{error, info, warn};   // Imports logging macros error and info.
use prost::Message;   //Imports the Message trait for encoding and decoding protocol buffer messages. 
use std::{
    io::{self, Read, Write},         //Imports I/O traits and types
    net::{SocketAddr, TcpStream, ToSocketAddrs},    //Imports networking types and traits.
    time::Duration,                //Imports the Duration type for handling timeouts
};

// TCP/IP Client: Defines a struct to represent a TCP client.
pub struct Client {
    ip: String,
    port: u32,
    timeout: Duration,
    retries: usize,
    max_retries: usize,
    stream: Option<TcpStream>,
  }

//Implementation of Client
impl Client {
     // Creates a new client instance and connects to the server
    pub fn new(ip: &str, port: u32, timeout_ms: u64, max_retries: usize) -> Self {   
        Client {
            ip: ip.to_string(),   //Converts the IP address to a string.
            port,                 //Sets the port number.
            timeout: Duration::from_millis(timeout_ms),         //Converts the timeout from milliseconds to a Duration.
            retries: 0,
            max_retries,
            stream: None,                                  //Initializes the stream as None.
        }
    }

    //Connect Method: connect the client to the server
    pub fn connect(&mut self) -> io::Result<()> {
        info!("Connecting to {}:{}", self.ip, self.port);

        // Resolve the address
        let address = format!("{}:{}", self.ip, self.port);        // Formats the IP and port into a single string
        let socket_addrs: Vec<SocketAddr> = address.to_socket_addrs()?.collect();   //Resolves the address to a list of SocketAddr instances

        if socket_addrs.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid IP or port",
            ));
        }

        // Connect to the server with a timeout
        let stream = TcpStream::connect_timeout(&socket_addrs[0], self.timeout)?;      
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;
        self.stream = Some(stream);       //Stores the connected TcpStream.

        info!("Connected to the server!");
        Ok(())
    }

    //Disconnect Method: disconnect the client
    pub fn disconnect(&mut self) -> io::Result<()> {
        if let Some(stream) = self.stream.take() {     //Takes ownership of the stream, setting it to None.
            stream.shutdown(std::net::Shutdown::Both)?;    //huts down the connection.
        }

        info!("Disconnected from the server!");    //Returns an error if the shutdown fails.
        Ok(())
    }

    // generic message to send message to the server
    //Send Method
    pub fn send(&mut self,  content: &str) -> io::Result<()> {
        if let Some(ref mut stream) = self.stream {
            
            // Construct and encode the EchoMessage
            let message = EchoMessage {
                content: content.to_string(),
            };
        
            // Encode the message to a buffer
            let mut buffer = Vec::new();
            message.encode(&mut buffer);      // Encodes the message into a buffer

            // Send the buffer to the server
            stream.write_all(&buffer)?;     //Writes the buffer to the stream
            stream.flush()?;      //Ensures all data is sent.

            info!("Sent message: {:?}", message);     
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "No active connection",                //Returns an error if there is no active connection or if sending fails.
            ))
        }
    }
    
    //Receive Method:Receives a message from the server
    pub fn receive(&mut self) -> io::Result<EchoMessage> {
        if let Some(ref mut stream) = self.stream {
            info!("Receiving message from the server...");
            let mut buffer = vec![0u8; 512];
            let bytes_read = stream.read(&mut buffer)?;          //eads data from the stream into a buffer.
            if bytes_read == 0 {          //Checks if the server has disconnected.
                warn!("Server disconnected.");
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "Server disconnected",
                ));
            }

            info!("Received {} bytes from the server", bytes_read);

            // Decode the received message
            EchoMessage::decode(&buffer[..bytes_read]).map_err(|e| {
                error!("Failed to decode message: {}", e);
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to decode ServerMessage: {}", e),        //Returns an error if there is no active connection, if reading fails, or if decoding fails.
                )
            })
        } else {
            error!("No active connection");
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "No active connection",
            ))
        }


        
    // Send and receive with retries : Combines sending and receiving into a robust operation with retries.
    pub fn send_and_receive(&mut self, content: &str) -> io::Result<EchoMessage> {
        while self.retries < self.max_retries {
            match self.send(content).and_then(|_| self.receive()) {
                Ok(response) => {
                    self.retries = 0; // Reset retries on success
                    return Ok(response);
                }
                Err(e) => {
                    self.retries += 1;
                    warn!(
                        "Attempt {} failed: {}. Retrying...",
                        self.retries, e
                    );

                    if self.retries >= self.max_retries {
                        error!("Max retries reached. Giving up.");
                        return Err(e);
                    }
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::Other,
            "Unhandled error in send_and_receive",
        ))
    }    }    
}
