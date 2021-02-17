use crate::{command::external::FrontendMessage, command::Command};
use crossbeam_channel::{Receiver, Sender};
use log::error;
use serde::{Deserialize, Serialize};

pub struct Cell {
  #[allow(dead_code)]
  id: String,
  current_dir: String,
  value: String,

  pub tsfn: ThreadsafeFunctionType,
  pub sender: Sender<CellChannel>,
  pub receiver: Receiver<CellChannel>,
}

impl Cell {
  pub fn new(
    props: CellProps,
    tsfn: ThreadsafeFunctionType,
    sender: Sender<CellChannel>,
    receiver: Receiver<CellChannel>,
  ) -> Self {
    let CellProps {
      id,
      current_dir,
      value,
    } = props;

    Self {
      id,
      current_dir,
      value,

      tsfn,
      sender,
      receiver,
    }
  }

  pub fn current_dir(&self) -> &str {
    self.current_dir.as_ref()
  }

  pub fn run(self) {
    self.send(ServerMessage::status(Status::Running));

    // once operators (|, &&, ||) are introduced, this could become Vec<Command>
    let command = parse_value(&(self.value), &(self.current_dir));

    if let Err(err) = command.execute(self) {
      error!("Error while executing command: {}", err);
    };
  }

  pub fn send(&self, message: ServerMessage) {
    tsfn_send(&self.tsfn, message);
  }
}

type ThreadsafeFunctionType =
  napi::threadsafe_function::ThreadsafeFunction<std::vec::Vec<ServerMessage>>;

pub fn tsfn_send(tsfn: &ThreadsafeFunctionType, message: ServerMessage) {
  tsfn.call(
    Ok(vec![message]),
    napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
  );
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CellProps {
  id: String,
  value: String,
  current_dir: String,
}

pub enum CellChannel {
  FrontendMessage(FrontendMessage),
  Exit,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerMessage {
  #[serde(flatten)]
  data: Data,
  #[serde(skip_serializing_if = "Option::is_none")]
  action: Option<Action>,
}

pub type Action = Vec<(String, String)>;

impl ServerMessage {
  pub fn new(data: Data, action: Option<Action>) -> Self {
    Self { data, action }
  }

  pub fn status(status: Status) -> Self {
    Self {
      data: Data::Status(status),
      action: None,
    }
  }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Data {
  Text(Vec<u8>), // pty (external commands) is always text, except when it starts with "<Termy" OR if it's being piped and can be parsed as JSON
  Api(String),
  Mdx(String), // same thing as api with cosmetic enhancements
  Status(Status),
  Error(String),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Status {
  Running,
  Success,
  Error,
}

fn parse_value(value: &str, current_dir: &str) -> Command {
  let mut tokens = tokenize_value(value);

  Command::new(tokens.remove(0), tokens, current_dir)
}

fn tokenize_value(value: &str) -> Vec<String> {
  let mut inside_quotes = false;
  let mut tokens: Vec<String> = vec![];
  let mut token = String::new();

  for c in value.chars() {
    if c == '"' {
      inside_quotes = !inside_quotes;
    } else if c.is_whitespace() && !inside_quotes {
      tokens.push(token.clone());
      token.clear();
    } else {
      token.push(c);
    }
  }
  tokens.push(token);

  tokens
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tokenizes_value() {
    assert_eq!(
      tokenize_value("command arg1 arg2"),
      vec![
        "command".to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
      ]
    );

    assert_eq!(
      tokenize_value("create \"Whitespace inside quotes yay'\""),
      vec![
        "create".to_string(),
        "Whitespace inside quotes yay'".to_string()
      ]
    );

    assert_eq!(
      tokenize_value("diskutil \"\"WIN10\"\""),
      vec!["diskutil".to_string(), "WIN10".to_string()]
    );
  }
}
