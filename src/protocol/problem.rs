use serde::{Deserialize, Serialize};
use crate::protocol::Command;

#[derive(Serialize, Deserialize)]
pub struct ProblemCommand {
    problem: Problem,
    message: String
}

impl ProblemCommand {

    pub fn new(problem: Problem, message: String) -> Self {
        Self {
            problem,
            message
        }
    }

}

impl Command for ProblemCommand {

    fn name() -> &'static str { "init" }

}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Problem {
    AuthenticationFailed,
    InternalError
}