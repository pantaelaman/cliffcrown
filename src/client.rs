use std::{os::unix::net::UnixStream, sync::Arc};

use egui::Context;
use either::Either::{self, Left, Right};
use greetd_ipc::codec::SyncCodec;
use tokio::sync::oneshot;

const GREETD_SOCK_ENV: &'static str = "GREETD_SOCK";

#[derive(Debug)]
pub enum ClientError {
  MissingEnvVar,
  FailedSocketConnection(std::io::Error),
  FailedSocketWrite(greetd_ipc::codec::Error),
  FailedSocketRead(greetd_ipc::codec::Error),
  GenericError(String),
  AuthError(String),
}

impl std::fmt::Display for ClientError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::MissingEnvVar => write!(
        f,
        "GREETD_SOCK environment variable not found. Is greetd running?"
      ),
      Self::FailedSocketConnection(e) => {
        write!(f, "couldn't connect to the greetd socket: {}", e)
      }
      Self::FailedSocketWrite(e) => {
        write!(f, "couldn't write message to socket: {}", e)
      }
      Self::FailedSocketRead(e) => {
        write!(f, "couldn't read message from socket: {}", e)
      }
      Self::GenericError(e) => {
        write!(f, "generic greetd error: {}", e)
      }
      Self::AuthError(e) => {
        write!(f, "authentication error: {}", e)
      }
    }
  }
}

impl std::error::Error for ClientError {}

#[derive(Debug, Clone)]
pub enum AuthPrompt {
  Input { prompt: String, secret: bool },
  Info { note: String },
  Error { note: String },
}

#[derive(Debug)]
pub struct Client {
  stream: UnixStream,
}

#[derive(Debug)]
pub struct ActiveClient {
  stream: UnixStream,
}

#[derive(Debug)]
pub struct PromptingClient {
  stream: UnixStream,
  pub prompt: AuthPrompt,
}

#[derive(Debug)]
pub struct SuccessfulClient {
  stream: UnixStream,
}

impl Client {
  pub fn new() -> Result<Self, ClientError> {
    let sock =
      std::env::var(GREETD_SOCK_ENV).map_err(|_| ClientError::MissingEnvVar)?;

    let stream = UnixStream::connect(sock)
      .map_err(|e| ClientError::FailedSocketConnection(e))?;

    Ok(Self { stream })
  }

  pub fn create_session(
    mut self,
    username: String,
  ) -> Result<ActiveClient, (ClientError, Self)> {
    let request = greetd_ipc::Request::CreateSession { username };
    if let Err(e) = request.write_to(&mut self.stream) {
      return Err((ClientError::FailedSocketWrite(e), self));
    }

    Ok(ActiveClient {
      stream: self.stream,
    })
  }
}

impl ActiveClient {
  pub fn next(
    mut self,
  ) -> Result<Either<PromptingClient, SuccessfulClient>, (ClientError, Client)>
  {
    let response = match greetd_ipc::Response::read_from(&mut self.stream) {
      Ok(r) => r,
      Err(e) => {
        return Err((
          ClientError::FailedSocketRead(e),
          Client {
            stream: self.stream,
          },
        ));
      }
    };

    match response {
      greetd_ipc::Response::Success => Ok(Right(SuccessfulClient {
        stream: self.stream,
      })),
      greetd_ipc::Response::Error {
        error_type,
        description,
      } => Err((
        match error_type {
          greetd_ipc::ErrorType::Error => {
            ClientError::GenericError(description)
          }
          greetd_ipc::ErrorType::AuthError => {
            ClientError::AuthError(description)
          }
        },
        Client {
          stream: self.stream,
        },
      )),
      greetd_ipc::Response::AuthMessage {
        auth_message_type,
        auth_message,
      } => Ok(Left(PromptingClient {
        stream: self.stream,
        prompt: match auth_message_type {
          greetd_ipc::AuthMessageType::Visible => AuthPrompt::Input {
            prompt: auth_message,
            secret: false,
          },
          greetd_ipc::AuthMessageType::Secret => AuthPrompt::Input {
            prompt: auth_message,
            secret: true,
          },
          greetd_ipc::AuthMessageType::Info => {
            AuthPrompt::Info { note: auth_message }
          }
          greetd_ipc::AuthMessageType::Error => {
            AuthPrompt::Error { note: auth_message }
          }
        },
      })),
    }
  }

  pub fn cancel(mut self) -> (Client, Option<ClientError>) {
    let request = greetd_ipc::Request::CancelSession;
    let error = request
      .write_to(&mut self.stream)
      .map_err(|e| ClientError::FailedSocketWrite(e))
      .err();
    (
      Client {
        stream: self.stream,
      },
      error,
    )
  }
}

impl PromptingClient {
  pub fn next(
    mut self,
    answer: Option<String>,
  ) -> Result<ActiveClient, (ClientError, Self)> {
    let request =
      greetd_ipc::Request::PostAuthMessageResponse { response: answer };
    if let Err(e) = request.write_to(&mut self.stream) {
      return Err((ClientError::FailedSocketWrite(e), self));
    }

    Ok(ActiveClient {
      stream: self.stream,
    })
  }

  pub fn cancel(mut self) -> (Client, Option<ClientError>) {
    let request = greetd_ipc::Request::CancelSession;
    let error = request
      .write_to(&mut self.stream)
      .map_err(|e| ClientError::FailedSocketWrite(e))
      .err();
    (
      Client {
        stream: self.stream,
      },
      error,
    )
  }
}

impl SuccessfulClient {
  pub fn finish(
    mut self,
    command: Vec<String>,
    environment: Vec<String>,
  ) -> Result<(), (ClientError, Self)> {
    let request = greetd_ipc::Request::StartSession {
      cmd: command,
      env: environment,
    };
    request
      .write_to(&mut self.stream)
      .map_err(|e| (ClientError::FailedSocketWrite(e), self))
  }
}

pub type UsernamePacket = (String, oneshot::Sender<StatePacket>);
pub type PromptResponsePacket = (Option<String>, oneshot::Sender<StatePacket>);

#[derive(Debug)]
pub enum StatePacket {
  Prompt {
    prompt: AuthPrompt,
    response_sender: oneshot::Sender<PromptResponsePacket>,
  },
  Success {
    command_sender: oneshot::Sender<Vec<String>>,
  },
}

pub struct ClientManager {
  receiver: oneshot::Receiver<UsernamePacket>,
  client: Client,
}

impl ClientManager {
  pub fn new() -> Result<(oneshot::Sender<UsernamePacket>, Self), ClientError> {
    let (sender, receiver) = oneshot::channel();
    Ok((
      sender,
      ClientManager {
        receiver,
        client: Client::new()?,
      },
    ))
  }

  pub async fn run(self) -> Result<(), ClientError> {
    let ClientManager {
      receiver: username_receiver,
      client,
    } = self;

    let (username, mut responder) = username_receiver.await.unwrap();
    let mut active_client =
      client.create_session(username).map_err(|(e, _)| e)?;

    loop {
      match active_client.next().map_err(|(e, _)| e)? {
        Left(prompting_client) => {
          let (prompt_sender, prompt_receiver) = oneshot::channel();
          responder.send(StatePacket::Prompt {
            prompt: prompting_client.prompt.clone(),
            response_sender: prompt_sender,
          });
          let (prompt_response, new_responder) = prompt_receiver.await.unwrap();
          responder = new_responder;
          active_client =
            prompting_client.next(prompt_response).map_err(|(e, _)| e)?;
        }
        Right(successful_client) => {
          let (command_sender, command_receiver) = oneshot::channel();
          responder.send(StatePacket::Success { command_sender });
          let command = command_receiver.await.unwrap();
          successful_client
            .finish(command, vec![])
            .map_err(|(e, _)| e)?;
          return Ok(());
        }
      }
    }
  }
}
