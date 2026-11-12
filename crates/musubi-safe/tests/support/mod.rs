//! Shared external-Anvil process fixture for Safe integration tests.

use std::{
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(crate) struct Anvil {
    child: Child,
    pub(crate) endpoint: url::Url,
}

impl Anvil {
    pub(crate) fn spawn() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("reserve Anvil port");
        let port = listener.local_addr().expect("read reserved port").port();
        drop(listener);
        let binary = std::env::var_os("MUSUBI_ANVIL").unwrap_or_else(|| "anvil".into());
        let child = Command::new(binary)
            .args([
                "--port",
                &port.to_string(),
                "--chain-id",
                "31337",
                "--silent",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Anvil is required; set MUSUBI_ANVIL when it is not on PATH");
        let deadline = Instant::now() + Duration::from_secs(10);
        while TcpStream::connect(("127.0.0.1", port)).is_err() {
            assert!(Instant::now() < deadline, "Anvil did not become ready");
            thread::sleep(Duration::from_millis(25));
        }
        Self {
            child,
            endpoint: format!("http://127.0.0.1:{port}")
                .parse()
                .expect("local endpoint is valid"),
        }
    }
}

impl Drop for Anvil {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
