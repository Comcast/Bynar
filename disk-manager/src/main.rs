extern crate api;
extern crate protobuf;
extern crate zmq;

use std::thread;
use std::time::Duration;

use zmq::Result as ZmqResult;

/*
 Server that manages disks
 */

fn listen() -> ZmqResult<()> {
    let context = zmq::Context::new();
    let responder = context.socket(zmq::REP).unwrap();

    assert!(responder.bind("tcp://*:5555").is_ok());

    let mut msg = zmq::Message::new()?;
    loop {
        responder.recv(&mut msg, 0).unwrap();
        println!("Received {}", msg.as_str().unwrap());
        thread::sleep(Duration::from_millis(1000));
        responder.send("World".as_bytes(), 0).unwrap();
    }
}

fn main() {
    // Hello world
    listen();
}
