use std::io::{BufRead, Read, Write};
use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize)]
struct CommandWrapper<T> {
    command: String,

    #[serde(flatten)]
    inner: T
}

impl<T> CommandWrapper<T>
where T: Command {

    fn new(inner: T) -> Self {
        Self {
            command: T::name().to_string(),
            inner
        }
    }

    fn to_inner(self) -> T {
        self.inner
    }

}

pub trait Command: serde::Serialize + serde::de::DeserializeOwned {

    fn name() -> &'static str;

    fn send(self) -> Result<(), ()> {
        let command_string = match serde_json::to_string(&CommandWrapper::new(self)) {
            Ok(string) => string,
            Err(err) => {
                eprintln!("Error while serializing command: {}", err);
                return Err(())
            }
        };

        print!("{}\n\n{}", command_string.len() + 1, command_string);

        match std::io::stdout().flush() {
            Ok(_) => {},
            Err(err) => {
                eprintln!("Error while flushing stdout: {}", err);
                return Err(())
            }
        }

        Ok(())
    }

    fn receive() -> Result<Self, ()> {
        let mut stdin = std::io::stdin().lock();

        let mut length_string = String::new();

        match stdin.read_line(&mut length_string) {
            Ok(_) => {},
            Err(err) => {
                eprintln!("Error while reading length line: {}", err);
                return Err(())
            }
        };

        let length: usize = match length_string.trim().parse() {
            Ok(x) => x,
            Err(err) => {
                eprintln!("Error while parsing command length: {}", err);
                return Err(());
            }
        };

        let mut command_buffer = vec![0; length];

        match stdin.read_exact(&mut command_buffer) {
            Ok(_) => {},
            Err(err) => {
                eprintln!("Error while reading command: {}", err);
                return Err(());
            }
        };

        if command_buffer.len() != length {
            eprintln!("Invalid command buffer length: received '{}', expected: '{}'", command_buffer.len(), length);
            return Err(());
        }

        if command_buffer.remove(0) != b'\n' {
            eprintln!("Command buffer does not start with newline");
            return Err(());
        }

        let wrapped_command: CommandWrapper<Self> = match serde_json::from_slice(&command_buffer) {
            Ok(x) => x,
            Err(err) => {
                eprintln!("Error while deserializing command: {}", err);
                return Err(());
            }
        };

        if wrapped_command.command != Self::name() {
            eprintln!("Unexpected command received: received '{}', expected: '{}'", wrapped_command.command, Self::name());
            return Err(());
        }

        return Ok(wrapped_command.to_inner())
    }

}