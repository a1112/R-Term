use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use rterm_web::{WebServer, WebServerConfig};

const DEFAULT_LISTEN: SocketAddr = SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 7798);
const DEFAULT_WEB_ROOT: &str = "web/dist";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = Options::parse(std::env::args().skip(1))?;
    let config = WebServerConfig {
        listen: options.listen,
        web_root: options.web_root,
        max_sessions: options.max_sessions,
        allowed_origin: options.allowed_origin,
    };
    let server = WebServer::bind(config).await?;
    println!("R-Term Web terminal: {}", server.bootstrap_url());
    println!("Press Ctrl+C to stop.");
    server.run_until_shutdown().await?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct Options {
    listen: SocketAddr,
    web_root: PathBuf,
    max_sessions: usize,
    allowed_origin: Option<String>,
}

impl Options {
    fn parse<I>(arguments: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self {
            listen: DEFAULT_LISTEN,
            web_root: PathBuf::from(DEFAULT_WEB_ROOT),
            max_sessions: rterm_web::server::DEFAULT_MAX_SESSIONS,
            allowed_origin: None,
        };
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--listen" => {
                    let value = arguments
                        .next()
                        .ok_or_else(|| "--listen requires HOST:PORT".to_owned())?;
                    options.listen = value
                        .parse()
                        .map_err(|_| format!("invalid --listen address: {value}"))?;
                }
                "--web-root" => {
                    let value = arguments
                        .next()
                        .ok_or_else(|| "--web-root requires a directory".to_owned())?;
                    options.web_root = PathBuf::from(value);
                }
                "--max-sessions" => {
                    let value = arguments
                        .next()
                        .ok_or_else(|| "--max-sessions requires a positive integer".to_owned())?;
                    options.max_sessions = value
                        .parse::<usize>()
                        .map_err(|_| format!("invalid --max-sessions value: {value}"))?;
                    if options.max_sessions == 0 {
                        return Err("--max-sessions must be greater than zero".to_owned());
                    }
                }
                "--allowed-origin" => {
                    let value = arguments
                        .next()
                        .ok_or_else(|| "--allowed-origin requires an origin".to_owned())?;
                    options.allowed_origin = Some(value);
                }
                "--help" | "-h" => {
                    println!(
                        "Usage: rterm-web [--listen HOST:PORT] [--web-root DIR] [--max-sessions N] \
                         [--allowed-origin ORIGIN]"
                    );
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}")),
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use super::{DEFAULT_LISTEN, DEFAULT_WEB_ROOT, Options};

    #[test]
    fn options_use_loopback_defaults() {
        let options = Options::parse(Vec::<String>::new()).unwrap();
        assert_eq!(options.listen, DEFAULT_LISTEN);
        assert_eq!(options.web_root.to_string_lossy(), DEFAULT_WEB_ROOT);
        assert_eq!(
            options.max_sessions,
            rterm_web::server::DEFAULT_MAX_SESSIONS
        );
        assert_eq!(options.allowed_origin, None);
    }

    #[test]
    fn options_parse_server_overrides() {
        let options = Options::parse([
            "--listen".to_owned(),
            "127.0.0.1:9000".to_owned(),
            "--web-root".to_owned(),
            "target/web".to_owned(),
            "--max-sessions".to_owned(),
            "3".to_owned(),
            "--allowed-origin".to_owned(),
            "http://localhost:5173".to_owned(),
        ])
        .unwrap();
        assert_eq!(
            options.listen,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9000)
        );
        assert_eq!(options.web_root.to_string_lossy(), "target/web");
        assert_eq!(options.max_sessions, 3);
        assert_eq!(
            options.allowed_origin.as_deref(),
            Some("http://localhost:5173")
        );
    }

    #[test]
    fn options_reject_zero_sessions() {
        let error = Options::parse(["--max-sessions".to_owned(), "0".to_owned()]).unwrap_err();
        assert!(error.contains("greater than zero"));
    }
}
