mod protocol;
mod oidc;

use std::fmt::{Display};
use std::path::{PathBuf};
use rand::{distributions::DistString};
use protocol::AuthChallengeCommand;
use crate::protocol::{AuthResponseCommand, Command, Problem, ProblemCommand};

static COCKPIT_SSH_COMMAND: &'static str = "/usr/libexec/cockpit-ssh";

fn main() -> Result<(), ()> {
    let mut args = std::env::args();

    let Some(_) = args.next() else {
        internal_problem("Missing first argument", "");
        return Err(());
    };

    let Some(host) = args.next() else {
        internal_problem("Missing host argument", "");
        return Err(());
    };

    let client_id = env_var_or_error("COCKPIT_OIDC_CLIENT_ID")?;
    let client_secret = env_var_or_error("COCKPIT_OIDC_CLIENT_SECRET")?;
    let issuer_url = env_var_or_error("COCKPIT_OIDC_ISSUER_URL")?;
    let login_url = env_var_or_error("COCKPIT_OIDC_LOGIN_URL")?;
    let ssh_keys_path = env_var_or_error("COCKPIT_OIDC_SSH_KEYS_PATH")?;

    eprintln!("Performing OIDC authentication for host '{}'", host);

    let cookie = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

    match AuthChallengeCommand::new("*".to_string(), cookie.to_string()).send() {
        Ok(_) => {},
        Err(_) => {
            internal_problem("Failed to send auth challenge command", "");
            return Err(());
        }
    }

    let reply = match AuthResponseCommand::receive() {
        Ok(reply) => reply,
        Err(_) => {
            internal_problem("Did not receive auth response", "");
            return Err(());
        }
    };

    let Some(received_cookie) = reply.cookie() else {
        internal_problem("No cookie received", "");
        return Err(());
    };

    if *received_cookie != cookie {
        internal_problem("Invalid cookie received", "");
        return Err(());
    }

    let authorization = reply.response();

    if !authorization.starts_with("Bearer ") {
        internal_problem("Invalid authorization received", "");
        return Err(());
    }

    let code = match authorization.strip_prefix("Bearer ") {
        Some(x) => x,
        None => {
            internal_problem("Failed to parse authorization", "");
            return Err(());
        }
    };

    let username = match oidc::exchange(code, &client_id, &client_secret, &issuer_url, &redirect_url(&login_url, &host)) {
        Ok(x) => x,
        Err(err) => {
            authentication_failure("Failed to exchange code", err);
            return Err(());
        }
    };

    let mut user_key_path = PathBuf::new();
    user_key_path.push(ssh_keys_path);
    user_key_path.push(&username);

    if !user_key_path.exists() {
        authentication_failure("Could not find user SSH key", "");
        return Err(());
    }

    eprintln!("Using SSH key: {}", user_key_path.to_string_lossy().to_string());

    match std::process::Command::new("ssh-add")
        .arg("-t 30")
        .arg(user_key_path.into_os_string())
        .output() {
        Ok(output) => { eprintln!("ssh-add {}", output.status) },
        Err(err) => {
            internal_problem("Error while adding SSH key", err);
        }
    }

    match AuthResponseCommand::new("ssh-agent".to_string(), None).send() {
        Ok(_) => {},
        Err(_) => { return Err(()) }
    }

    let connection_string;

    if host.contains("@") {
        connection_string = host;
    } else {
        connection_string = format!("{}@{}", username, host);
    }

    let err = exec::execvp(COCKPIT_SSH_COMMAND, &[COCKPIT_SSH_COMMAND, &connection_string]);
    internal_problem("SSH exited with error", err);

    Ok(())
}

fn env_var_or_error(key: &str) -> Result<String, ()> {
    match std::env::var(key) {
        Ok(x) => Ok(x),
        Err(err) => {
            internal_problem(&format!("Missing environment variable '{}'", key), err);
            Err(())
        }
    }
}

fn redirect_url(login_url: &str, host: &str) -> String {
    // A different host can be selected using <LOGIN_URL>/=host. This host is then passed on as the
    // first argument. If no host is provided, '127.0.0.1' is passed. Since this is the most common
    // scenario and the redirect URL is required, it is assumed that no host was provided. If the
    // host '127.0.0.1' was provided explicitly, this will result in an authentication error.
    return if host == "127.0.0.1" {
        login_url.to_string()
    } else {
        if login_url.ends_with("/") {
            format!("{}={}", login_url, host)
        } else {
            format!("{}/={}", login_url, host)
        }
    }
}

fn internal_problem(message: &str, error: impl Display) {
    eprintln!("Internal error: {} {}", message, error);
    match ProblemCommand::new(Problem::InternalError, message.to_string()).send() {
        Ok(_) => {},
        Err(_) => {
            eprintln!("Error while sending internal error problem command");
        }
    }
}

fn authentication_failure(message: &str, error: impl Display) {
    eprintln!("Authentication failure: {} {}", message, error);
    match ProblemCommand::new(Problem::AuthenticationFailed, message.to_string()).send() {
        Ok(_) => {},
        Err(_) => {
            eprintln!("Error while sending authentication failure problem command");
        }
    }
}

