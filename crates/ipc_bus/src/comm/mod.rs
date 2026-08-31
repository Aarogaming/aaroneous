use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};

pub struct AgentBus {
    pipe_path: String,
}

impl AgentBus {
    pub fn new(pipe_name: &str) -> Self {
        Self {
            pipe_path: format!(r"\\.\pipe\{}", pipe_name),
        }
    }

    pub fn create_server(&self) -> Result<NamedPipeServer> {
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&self.pipe_path)
            .with_context(|| format!("Failed to create named pipe server at {}", self.pipe_path))?;
        Ok(server)
    }

    pub async fn send_signal(&self, message: &str) -> Result<()> {
        let mut client = ClientOptions::new()
            .open(&self.pipe_path)
            .with_context(|| format!("Failed to connect to named pipe at {}", self.pipe_path))?;

        client.write_all(message.as_bytes()).await?;
        client.flush().await?;
        Ok(())
    }
}
