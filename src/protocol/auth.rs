
use serde::{Deserialize, Serialize};
use super::Command;

#[derive(Serialize, Deserialize)]
pub struct AuthChallengeCommand {
    challenge: String,
    cookie: String
}

impl AuthChallengeCommand {

    pub fn new(challenge: String, cookie: String) -> Self {
        Self {
            challenge,
            cookie
        }
    }

}

impl Command for AuthChallengeCommand {
    fn name() -> &'static str { "authorize" }
}

#[derive(Serialize, Deserialize)]
pub struct AuthResponseCommand {
    response: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cookie: Option<String>
}

impl AuthResponseCommand {

    pub fn new(response: String, cookie: Option<String>) -> Self {
        Self {
            response,
            cookie
        }
    }

    pub fn response(&self) -> &str { self.response.as_str() }
    pub fn cookie(&self) -> Option<&String> { self.cookie.as_ref() }

}

impl Command for AuthResponseCommand {
    fn name() -> &'static str { "authorize" }
}