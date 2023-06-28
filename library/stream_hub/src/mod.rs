trait SessionHandler {}
struct StreamsHub {}

struct Stream {
    identifier: StreamIdentifier,
    session_handler: SessionHandler,
}

enum StreamIdentifier {
    Rtmp {
        app_name: String,
        stream_name: String,
    },
    Rtsp {
        stream_name: String,
    },
}
