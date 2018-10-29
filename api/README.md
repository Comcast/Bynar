# API
This crate holds the protobuf api specification.  All crates in this workspace share this.
All communication between components happen over protocol buffers.  Why?  Because any language
that has a protocol buffers library ( There's a lot of them ) can support talking to this 
tool over a fast and efficient protocol.  All communication between clients and bynar can optionally
be encrypted using curve encryption from zeromq.  

ZeroMQ has the concept of a [request and reply tcp socket](https://rfc.zeromq.org/spec:28/REQREP/).  The
purpose of this is to make network communication easier.  For example a client cannot issue 2 requests in 
a row and a server cannot issue 2 replies in a row.  It is request then reply, request then reply.  ZeroMQ
enforces that. 

The protocol works like this:
1. Client opens an optionally curve encrypted ZeroMQ socket to Bynar port 5555
2. Client makes a Request [operation](https://github.com/Comcast/Bynar/blob/master/api/protos/service.proto#L130) from the API
   1. Currently a minimal set of operations can be performed but that will expand in the future.
   2. Please create a pull request if you'd like to Bynar to perform other operations.
3. Bynar makes a [Response](https://github.com/Comcast/Bynar/blob/master/api/protos/service.proto#L71) operation and sends it back to the client.  
4. The client then unpacks the server response and evaluates what to do next.  The client
can send another Request or it can stop.
Examples of how the protocol works in rust can be found [here](https://github.com/Comcast/Bynar/blob/master/helpers/src/lib.rs#L71).
