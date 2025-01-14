
//IMPORTS
use embedded_recruitment_task::{                            //Imports various message types (client_message, server_message, AddRequest, EchoMessage) and the Server struct from the embedded_recruitment_task crate.
    message::{client_message, server_message, AddRequest, EchoMessage},
    server::Server,
};
use std::{        //Imports synchronization primitives (Arc) and threading utilities (thread, JoinHandle).
    sync::Arc,
    thread::{self, JoinHandle},
};

mod client;       //Imports the client module

fn setup_server_thread(server: Arc<Server>) -> JoinHandle<()> {           //Spawns a new thread to run the server, Uses an Arc (atomic reference counted) pointer to share ownership of the Server instance across threads.
    thread::spawn(move || {
        server.run().expect("Server encountered an error");   //Panics with a message if the server encounters an error
    })
}

//Creates a new Server instance and wraps it in an Arc
fn create_server() -> Arc<Server> {
    Arc::new(Server::new("localhost:8080").expect("Failed to start server"))             //Initializes the server to listen on localhost:8080, Panics with a message if the server fails to start.
}


//Tests the client's ability to connect and disconnect from the server.
#[test]
fn test_client_connection() {
    // Set up the server in a separate thread
    let server = create_server();                         //Creates the server.
    let handle = setup_server_thread(server.clone());     //Runs the server in a separate thread

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);            //Creates a new client instance
    assert!(client.connect().is_ok(), "Failed to connect to the server");     //Connects the client to the server.
    assert!(client.disconnect().is_ok(), "Failed to disconnect from the server");   //Disconnects the client from the server
   
    //Stop the server and wait for the Server thread to finish
    server.stop();
    assert!(handle.join().is_ok(),"Server thread panicked or failed to join" );
}

//Tests the client's ability to send and receive an echo message.
#[test]
fn test_client_echo_message() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Prepares an echo message with the content "Hello, World!"
    let mut echo_message = EchoMessage::default();
    echo_message.content = "Hello, World!".to_string();  
    let message = client_message::Message::EchoMessage(echo_message.clone());  //Wraps the echo message in a client message.

    // Send the message to the server
    assert!(client.send(message).is_ok(), "Failed to send message");

    // Receive the echoed message from server
    let response = client.receive();
    assert!(response.is_ok(), "Failed to receive response for EchoMessage");


       if let Some(server_message::Message::EchoMessage(echo)) = response.unwrap().message {
        //Asserts that the echoed message content matches the sent message content.    
        assert_eq!(                          
                echo.content, echo_message.content,
                "Echoed message content does not match"
            );
        }else {
            panic!("Expected EchoMessage, but received a different message");
        }
        
    // Disconnect the client
    assert!( client.disconnect().is_ok(), "Failed to disconnect from the server" );

    // Stop the server and wait for thread to finish
    server.stop();
    assert!(handle.join().is_ok(),"Server thread panicked or failed to join");
}


//Tests the client's ability to send and receive multiple echo messages.
#[test]
fn test_multiple_echo_messages() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Prepare a list of messages to be sent
    let messages = vec![
        "Hello, World!".to_string(),
        "How are you?".to_string(),
        "Goodbye!".to_string(),
    ];

    //Iterates over each message, sending and receiving it, and asserting that the echoed content matches the sent content
    for message_content in &messages {
        let mut echo_message = EchoMessage::default();
        echo_message.content = message_content.clone();
        let message = client_message::Message::EchoMessage(echo_message);

        // Send the message to the server
        assert!(client.send(message).is_ok(), "Failed to send message");

        // Receive the echoed message
        let response = client.receive();
        assert!(response.is_ok(), "Failed to receive response for EchoMessage");

        
           if let Some(server_message::Message::EchoMessage(echo)) = response.unwrap().message {
                assert_eq!(
                    echo.content, message_content,
                    "Echoed message content does not match"
                );
            }else{
                panic!("Expected EchoMessage, but received a different message");
            }
    }

    // Disconnect the client
    assert!( client.disconnect().is_ok(), "Failed to disconnect from the server" );

    // Stop the server and wait for thread to finish
    server.stop();
    assert!( handle.join().is_ok(), "Server thread panicked or failed to join" );
}

//Tests the ability of multiple clients to connect, send, and receive messages concurrently
#[test]
fn test_multiple_clients() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect multiple client instances
    let mut clients = vec![
        client::Client::new("localhost", 8080, 1000),
        client::Client::new("localhost", 8080, 1000),
        client::Client::new("localhost", 8080, 1000),
    ];

    for client in clients.iter_mut() {
        assert!(client.connect().is_ok(), "Failed to connect to the server");
    }

    // Prepare multiple messages
    let messages = vec![
        "Hello, World!".to_string(),
        "How are you?".to_string(),
        "Goodbye!".to_string(),
    ];

    // Send and receive multiple messages for each client
    for message_content in &messages {
        let mut echo_message = EchoMessage::default();
        echo_message.content = message_content.clone();
        let message = client_message::Message::EchoMessage(echo_message);

        //Iterates over each client, connecting, sending, and receiving messages, and asserting that the echoed content matches the sent content.
        for client in clients.iter_mut() {
            // Send the message to the server
            assert!(client.send(message.clone()).is_ok(), "Failed to send message");

            // Receive the echoed message
            let response = client.receive();
            assert!( response.is_ok(),"Failed to receive response for EchoMessage");

               if let Some(server_message::Message::EchoMessage(echo)) = response.unwrap().message {
                    assert_eq!(
                        echo.content, message_content,
                        "Echoed message content does not match"
                    );
                }else{
                    panic!("Expected EchoMessage, but received a different message");
                }
        }
    }

    // Disconnect the clients
    for client in clients.iter_mut() {
        assert!( client.disconnect().is_ok(), "Failed to disconnect from the server");
    }

    // Stop the server and wait for thread to finish
    server.stop();
    assert!( handle.join().is_ok(),"Server thread panicked or failed to join");
}


#[test]
fn test_client_add_request() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Prepare the message
    let mut add_request = AddRequest::default();
    add_request.a = 10;
    add_request.b = 20;
    let message = client_message::Message::AddRequest(add_request.clone());

    // Send the message to the server
    assert!(client.send(message).is_ok(), "Failed to send message");

    // Receive the response
    let response = client.receive();
    assert!( response.is_ok(), "Failed to receive response for AddRequest" );

       if let Some(server_message::Message::AddResponse(add_response)) = response.unwrap().message {
            assert_eq!(
                add_response.result,
                add_request.a + add_request.b,
                "AddResponse result does not match"
            );
        }
        else{
            panic!("Expected AddResponse, but received a different message");
        }

    // Disconnect the client
    assert!( client.disconnect().is_ok(),"Failed to disconnect from the server");

    // Stop the server and wait for thread to finish
    server.stop();
    assert!( handle.join().is_ok(), "Server thread panicked or failed to join" );
}

//Ensures the server handles invalid or malformed client messages gracefully
#[test]
fn test_invalid_message_handling() {
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Send an invalid message
    let invalid_message = client_message::Message::Unknown;
    assert!(client.send(invalid_message).is_err(), "Invalid message was not rejected");

    client.disconnect().expect("Failed to disconnect");
    server.stop();
    handle.join().expect("Server thread panicked or failed to join");
}

//Verifies the server can handle a large number of concurrent connections.
#[test]
fn test_stress_large_number_of_clients() {
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    let mut clients = Vec::new();
    for _ in 0..100 {
        let mut client = client::Client::new("localhost", 8080, 1000);
        assert!(client.connect().is_ok(), "Failed to connect client to server");
        clients.push(client);
    }

    for client in &mut clients {
        assert!(client.disconnect().is_ok(), "Failed to disconnect client");
    }

    server.stop();
    handle.join().expect("Server thread panicked or failed to join");
}

//Tests how the server and client handle situations where one of them fails to respond within the expected time frame
#[test]
fn test_timeout_handling() {
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    let mut client = client::Client::new("localhost", 8080, 1); // 1 ms timeout
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Delay to trigger timeout
    std::thread::sleep(std::time::Duration::from_millis(10));

    let response = client.receive();
    assert!(response.is_err(), "Timeout error was not triggered");

    client.disconnect().expect("Failed to disconnect");
    server.stop();
    handle.join().expect("Server thread panicked or failed to join");
}

//Checks the server's ability to handle multiple clients sending AddRequest messages simultaneously.
#[test]
fn test_concurrent_add_requests() {
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    let mut clients = vec![
        client::Client::new("localhost", 8080, 1000),
        client::Client::new("localhost", 8080, 1000),
    ];

    for client in &mut clients {
        assert!(client.connect().is_ok(), "Failed to connect to the server");
    }

    let add_requests = vec![(5, 7), (10, 20)];

    let handles: Vec<_> = clients
        .iter_mut()
        .enumerate()
        .map(|(i, client)| {
            let (a, b) = add_requests[i];
            thread::spawn(move || {
                let mut add_request = AddRequest::default();
                add_request.a = a;
                add_request.b = b;
                let message = client_message::Message::AddRequest(add_request);

                client.send(message).expect("Failed to send AddRequest");
                let response = client.receive().expect("Failed to receive response");

                if let Some(server_message::Message::AddResponse(add_response)) = response.message {
                    assert_eq!(
                        add_response.result,
                        a + b,
                        "Incorrect addition result"
                    );
                } else {
                    panic!("Expected AddResponse, but got different message");
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Client thread panicked");
    }

    for client in &mut clients {
        client.disconnect().expect("Failed to disconnect");
    }

    server.stop();
    handle.join().expect("Server thread panicked or failed to join");
}

//Verifies the behavior when the server shuts down while handling active clients or messages.
#[test]
fn test_graceful_shutdown_with_active_clients() {
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    server.stop();
    assert!(client.send(client_message::Message::EchoMessage(EchoMessage::default())).is_err(), "Client was able to send message to stopped server");

    handle.join().expect("Server thread panicked or failed to join");
}

//checks the server's behavior when there is a significant delay in sending or receiving a message.
#[test]
fn test_delayed_messages() {
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Prepare and send an echo message
    let mut echo_message = EchoMessage::default();
    echo_message.content = "Delayed message".to_string();
    let message = client_message::Message::EchoMessage(echo_message.clone());

    // Simulate a delay before sending the message
    std::thread::sleep(std::time::Duration::from_secs(2));
    assert!(
        client.send(message).is_ok(),
        "Failed to send message after delay"
    );

    // Simulate a delay before receiving the response
    std::thread::sleep(std::time::Duration::from_secs(2));
    let response = client.receive();
    assert!(
        response.is_ok(),
        "Failed to receive response for delayed message"
    );

    match response.unwrap().message {
        Some(server_message::Message::EchoMessage(echo)) => {
            assert_eq!(
                echo.content, echo_message.content,
                "Echoed message content does not match after delay"
            );
        }
        _ => panic!("Expected EchoMessage, but received a different message"),
    }

    client.disconnect().expect("Failed to disconnect");
    server.stop();
    handle.join().expect("Server thread panicked or failed to join");
}

//nsures the server behaves correctly when the maximum client limit is reached and new connections are refused.
#[test]
fn test_connection_refusal() {
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Assume the server allows a maximum of 2 clients (adjust if necessary)
    let mut clients = vec![
        client::Client::new("localhost", 8080, 1000),
        client::Client::new("localhost", 8080, 1000),
    ];

    // Connect the maximum allowed number of clients
    for client in &mut clients {
        assert!(client.connect().is_ok(), "Failed to connect a client to the server");
    }

    // Attempt to connect an additional client beyond the limit
    let mut additional_client = client::Client::new("localhost", 8080, 1000);
    assert!(
        additional_client.connect().is_err(),
        "Additional client was able to connect despite connection limit"
    );

    // Disconnect the clients and clean up
    for client in &mut clients {
        client.disconnect().expect("Failed to disconnect a client");
    }

    server.stop();
    handle.join().expect("Server thread panicked or failed to join");
}

//validate handling of extreme data sizes for the EchoMessage.content, ensures that the system can handle large payloads effectively without crashing or significant performance issues
#[test]
fn test_large_echo_message() {
    // Set up the server in a separate thread
    let server = create_server();
    let handle = setup_server_thread(server.clone());

    // Create and connect the client
    let mut client = client::Client::new("localhost", 8080, 1000);
    assert!(client.connect().is_ok(), "Failed to connect to the server");

    // Generate a large message content
    let large_message_content = "A".repeat(10_000_000);    //creates a 10MB string using "A".repeat(10_000_000)
    let mut echo_message = EchoMessage::default();
    echo_message.content = large_message_content.clone();
    let message = client_message::Message::EchoMessage(echo_message);

    // Send the large message to the server
    assert!(
        client.send(message).is_ok(),
        "Failed to send large message to the server"
    );

    // Receive the echoed large message from the server
    let response = client.receive();
    assert!(response.is_ok(), "Failed to receive response for large EchoMessage");

    match response.unwrap().message {
        Some(server_message::Message::EchoMessage(echo)) => {
            assert_eq!(
                echo.content, large_message_content,
                "Echoed message content does not match the large message"
            );
        }
        _ => panic!("Expected EchoMessage, but received a different message"),
    }

    // Disconnect the client
    assert!(
        client.disconnect().is_ok(),
        "Failed to disconnect from the server"
    );

    // Stop the server and wait for thread to finish
    server.stop();
    handle.join().expect("Server thread panicked or failed to join");
}
