use std::{
    io::{self, ErrorKind, Write},
    sync::{Arc, Mutex},
};

use interprocess::local_socket::{
    GenericNamespaced, Listener, ListenerOptions, Stream, prelude::*,
};

#[derive(Clone)]
pub struct IpcListener(pub Arc<Mutex<Listener>>);

const SOCKET_NAME: &str = "hozz.sock";

pub fn enforce_socket() -> io::Result<Option<Listener>> {
    let name = SOCKET_NAME.to_ns_name::<GenericNamespaced>()?;

    let opts = ListenerOptions::new().name(name.clone());

    match opts.create_sync() {
        Ok(listener) => Ok(Some(listener)),
        Err(e) if e.kind() == ErrorKind::AddrInUse => {
            tracing::debug!(
                "A running instance has been detected. Attempting to transfer focus..."
            );

            if let Ok(mut stream) = Stream::connect(name) {
                if let Err(e) = stream.write_all(b"WAKE_UP") {
                    tracing::error!(error = %e, "Failed to send wake-up message");
                } else {
                    tracing::info!("Sent wake-up message");
                }
            }

            Ok(None)
        },
        Err(e) => {
            tracing::error!(error = %e, "Unexpected error");
            Err(e)
        },
    }
}
