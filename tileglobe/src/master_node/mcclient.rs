struct MCClient {
    state: State,
}

impl MCClient {
    fn new() -> Self {
        Self {
            state: State::Handshaking
        }
    }
}

enum State {
    Handshaking,
    Status,
    Login,
    Configuration,
    Play,
}
