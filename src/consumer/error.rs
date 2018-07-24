error_chain! {
    errors {
        BroadcastError {
            description("could not send broadcast")
            display("could not send broadcast")
        }
        MultipleBroadcastError(broadcast_errors: Vec<Error>) {
            description("could not send multiple broadcasts")
            display("could not send multiple broadcasts: {:?}", broadcast_errors)
        }
    }

    foreign_links {
        Ws(::ws::Error);
        Serde(::serde_json::Error);
    }
}