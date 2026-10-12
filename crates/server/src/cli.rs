use control_core::error::{Error, Result};
#[derive(Debug, PartialEq)]
pub enum Command {
    Help,
    Serve { port: Option<u16> },
    CreateAdmin { username: String },
}
pub fn parse(args: &[String]) -> Result<Command> {
    if args.iter().any(|s| matches!(s.as_str(), "--help" | "-h")) {
        return Ok(Command::Help);
    }
    let command = args.first().map(String::as_str).unwrap_or("serve");
    let mut username = "admin".to_string();
    let mut port = None;
    let mut i = 1;
    while i < args.len() {
        let (flag, value, step) = if let Some((flag, value)) = args[i].split_once('=') {
            (flag, value, 1)
        } else {
            (
                args[i].as_str(),
                args.get(i + 1)
                    .ok_or(Error::new("invalid_arguments", 400))?
                    .as_str(),
                2,
            )
        };
        match flag {
            "--username" if command == "create-admin" => username = value.trim().to_lowercase(),
            "--port" if command == "serve" => {
                let parsed = value
                    .parse::<u16>()
                    .map_err(|_| Error::new("invalid_port", 400))?;
                if parsed == 0 {
                    return Err(Error::new("invalid_port", 400));
                }
                port = Some(parsed);
            }
            _ => return Err(Error::new("invalid_arguments", 400)),
        }
        i += step;
    }
    match command {
        "serve" => Ok(Command::Serve { port }),
        "create-admin" => Ok(Command::CreateAdmin { username }),
        _ => Err(Error::new("invalid_arguments", 400)),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_existing_commands() {
        assert_eq!(parse(&[]).unwrap(), Command::Serve { port: None });
        assert_eq!(
            parse(&["create-admin".into(), "--username".into(), "Admin".into()]).unwrap(),
            Command::CreateAdmin {
                username: "admin".into()
            }
        );
    }
    #[test]
    fn custom_port_supported() {
        assert_eq!(
            parse(&["serve".into(), "--port=8020".into()]).unwrap(),
            Command::Serve { port: Some(8020) }
        );
    }
    #[test]
    fn help() {
        assert_eq!(parse(&["--help".into()]).unwrap(), Command::Help);
    }
    #[test]
    fn rejects_unknown_flags() {
        assert!(parse(&["serve".into(), "--password".into(), "secret".into()]).is_err());
    }
    #[test]
    fn rejects_invalid_ports() {
        for port in ["0", "65536", "bad"] {
            assert!(parse(&["serve".into(), "--port".into(), port.into()]).is_err());
        }
    }
    #[test]
    fn rejects_missing_value() {
        assert!(parse(&["create-admin".into(), "--username".into()]).is_err());
    }
}
