use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, Stdio};
use std::time::Duration;

use tempfile::TempDir;

const MAX_WAIT: Duration = Duration::from_secs(2);

type Connection = embedded_io_adapters::tokio_1::FromTokio<tokio::net::TcpStream>;

#[derive(Debug)]
pub struct Mosquitto {
    process: Child,
    config: Config,
    _dir: TempDir,
}

impl Mosquitto {
    pub fn builder() -> Builder {
        Builder {
            config: Default::default(),
            dir: TempDir::new().unwrap(),
        }
    }

    pub async fn client(&self) -> miniqtt::Client<Connection, Vec<u8>> {
        self.client_with_buffer(Vec::new()).await
    }

    pub async fn client_with_buffer<B>(&self, buffer: B) -> miniqtt::Client<Connection, B> {
        let addr = ("127.0.0.1", self.config.port);
        let stream = wait_available(addr, MAX_WAIT).await.unwrap();
        let stream = embedded_io_adapters::tokio_1::FromTokio::new(stream);
        let connection = miniqtt::Connection::new(stream, buffer);

        miniqtt::Client::new(connection)
    }
}

impl Drop for Mosquitto {
    fn drop(&mut self) {
        let Some((stdout, stderr)) = wait_exit(&mut self.process, MAX_WAIT) else {
            return;
        };

        if !stdout.is_empty() {
            println!("---- Mosquitto Stdout:");
            println!("{stdout}");
        }
        if !stderr.is_empty() {
            println!("---- Mosquitto Stderr:");
            println!("{stderr}");
        }
    }
}

pub struct Builder {
    config: Config,
    dir: TempDir,
}

impl Builder {
    #[expect(unused, reason = "manual testing")]
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.config.credentials = Some((username.into(), password.into()));
        self
    }

    pub fn start(mut self) -> Mosquitto {
        let config_path = self.dir.path().join("mosquitto.conf");

        self.dir.disable_cleanup(true);

        self.config.write_to(&config_path).unwrap();

        let process = std::process::Command::new("mosquitto")
            .arg("-c")
            .arg(&config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        Mosquitto {
            process,
            config: self.config,
            _dir: self.dir,
        }
    }
}

#[derive(Debug)]
struct Config {
    port: u16,
    credentials: Option<(String, String)>,
}

impl Config {
    fn write_to(&self, to: &Path) -> io::Result<()> {
        let mut f = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(to)?;

        writeln!(f, "persistence false")?;
        writeln!(f, "listener {port}", port = self.port)?;

        if let Some((username, password)) = &self.credentials {
            let p = to.with_file_name("mosquitto.passwd");
            create_passwd(&p, username, password);
            writeln!(f, "password_file {path}", path = p.to_str().unwrap())?;
        } else {
            writeln!(f, "allow_anonymous true")?;
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: random_port(),
            credentials: None,
        }
    }
}

fn random_port() -> u16 {
    let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    socket.local_addr().unwrap().port()
}

fn create_passwd(p: &Path, username: &str, password: &str) {
    let status = std::process::Command::new("mosquitto_passwd")
        .arg("-c")
        .arg("-b")
        .arg(p)
        .arg(username)
        .arg(password)
        .status()
        .unwrap();

    assert!(status.success(), "failed to create mosquitto password");
}

fn wait_exit(process: &mut Child, max_wait: Duration) -> Option<(String, String)> {
    let mut total_wait = Duration::ZERO;
    let mut killed = false;

    loop {
        match process.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => (),
            Err(_) => return None,
        }

        if !killed {
            let _ = process.kill();
            killed = true;
        }

        let wait = Duration::from_millis(10);
        total_wait += wait;

        if total_wait >= max_wait {
            return None;
        }

        std::thread::sleep(wait);
    }

    let mut stdout = Vec::new();
    let _ = process.stdout.take()?.read_to_end(&mut stdout);
    let mut stderr = Vec::new();
    let _ = process.stderr.take()?.read_to_end(&mut stderr);

    Some((
        String::from_utf8_lossy(&stdout).into_owned(),
        String::from_utf8_lossy(&stderr).into_owned(),
    ))
}

async fn wait_available<A>(a: A, max_wait: Duration) -> io::Result<tokio::net::TcpStream>
where
    A: tokio::net::ToSocketAddrs,
    A: Copy,
{
    let mut total_wait = Duration::ZERO;

    loop {
        let err = match tokio::net::TcpStream::connect(a).await {
            Ok(s) => break Ok(s),
            Err(err) if err.kind() == io::ErrorKind::ConnectionRefused => err,
            Err(err) => return Err(err),
        };

        let wait = Duration::from_millis(10);
        total_wait += wait;

        if total_wait >= max_wait {
            return Err(err);
        }

        tokio::time::sleep(wait).await;
    }
}
