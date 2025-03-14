use std::borrow::Cow;

use reqwest::{
    self,
    header::{HeaderValue, CONTENT_TYPE},
};
use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    Image,
};
const NAME: &str = "mysten/sui-tools";
const TAG: &str = "staging-arm64";
const FULLNODE_RPC_PORT: ContainerPort = ContainerPort::Tcp(9000);
const FAUCET_PORT: ContainerPort = ContainerPort::Tcp(9123);

/// Community Testcontainers implementation for Sui blockchain.
///
/// This container wraps the Sui CLI command `sui start` with additional options to force
/// a new genesis, enable a faucet, set custom ports, and more.
///
/// # Usage
///
/// To use the latest Sui image:
/// ```rust,ignore
/// let node = Sui::latest().start().await?;
/// ```
///
/// You can also customize the container:
/// ```rust,ignore
/// let node = Sui::default()
///     .with_force_regenesis(true)
///     .with_network_config("/tmp/sui_config")
///     .with_faucet(true)
///     .with_faucet_port(6123)
///     .with_fullnode_rpc_port(8000)
///     .with_epoch_duration_ms(60000);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Sui {
    force_regenesis: bool,
    with_faucet: bool,
    faucet_port: Option<u16>,
    fullnode_rpc_port: Option<u16>,
    epoch_duration_ms: Option<u64>,
    network_config: Option<String>,
    tag: Option<String>,
}

impl Sui {
    /// Create a new Sui with the latest Sui image.
    pub fn staging() -> Self {
        Self {
            tag: Some("staging".to_string()),
            ..Default::default()
        }
    }

    /// Force regeneration of the genesis (i.e. start the network from scratch).
    pub fn with_force_regenesis(mut self, force: bool) -> Self {
        if force && self.network_config.is_some() {
            panic!("with_force_regenesis and with_network_config are mutually exclusive");
        }
        self.force_regenesis = force;
        self
    }

    /// Enable the faucet. To specify a custom faucet port, use [`with_faucet_port`].
    pub fn with_faucet(mut self, with_faucet: bool) -> Self {
        self.with_faucet = with_faucet;
        self
    }

    /// Set a custom faucet port (requires faucet to be enabled).
    pub fn with_faucet_port(mut self, port: u16) -> Self {
        self.faucet_port = Some(port);
        self
    }

    /// Set the fullnode RPC port. Defaults to 9000 if not specified.
    pub fn with_fullnode_rpc_port(mut self, port: u16) -> Self {
        self.fullnode_rpc_port = Some(port);
        self
    }

    /// Set the epoch duration in milliseconds.
    pub fn with_epoch_duration_ms(mut self, duration: u64) -> Self {
        self.epoch_duration_ms = Some(duration);
        self
    }

    /// Set the network config directory.
    pub fn with_network_config(mut self, config: impl Into<String>) -> Self {
        if self.force_regenesis {
            panic!("with_network_config and with_force_regenesis are mutually exclusive");
        }
        self.network_config = Some(config.into());
        self
    }
}

impl Image for Sui {
    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        let mut cmd = vec![];

        // The first argument must be "start" for the `sui start` command.
        cmd.push("start".to_string());

        if self.force_regenesis {
            cmd.push("--force-regenesis".to_string());
        }

        if let Some(ref config) = self.network_config {
            cmd.push("--network.config".to_string());
            cmd.push(config.clone());
        }

        if self.with_faucet {
            if let Some(port) = self.faucet_port {
                // When a port is provided, use the syntax `--with-faucet=<FAUCET_PORT>`
                cmd.push(format!("--with-faucet={}", port));
            } else {
                cmd.push("--with-faucet".to_string());
            }
        }

        if let Some(port) = self.fullnode_rpc_port {
            cmd.push("--fullnode-rpc-port".to_string());
            cmd.push(port.to_string());
        }

        if let Some(epoch) = self.epoch_duration_ms {
            cmd.push("--epoch-duration-ms".to_string());
            cmd.push(epoch.to_string());
        }

        cmd.into_iter().map(Cow::from)
    }

    fn entrypoint(&self) -> Option<&str> {
        Some("sui")
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        vec![("RUST_LOG".to_string(), "warning,sui_node=info".to_string())].into_iter()
    }
    fn expose_ports(&self) -> &[ContainerPort] {
        // If faucet is enabled, expose both fullnode and faucet ports.
        if self.with_faucet {
            static PORTS: [ContainerPort; 2] = [FULLNODE_RPC_PORT, FAUCET_PORT];
            &PORTS
        } else {
            static PORTS: [ContainerPort; 1] = [FULLNODE_RPC_PORT];
            &PORTS
        }
    }

    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        self.tag.as_deref().unwrap_or(TAG)
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::http(
            HttpWaitStrategy::new("/")
                .with_method(reqwest::Method::POST)
                .with_body(r#"{"jsonrpc":"2.0","method":"sui_getChainIdentifier","id":1}"#)
                .with_header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .with_expected_status_code(200_u16),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sui::Sui as SuiImage, testcontainers::runners::AsyncRunner};
    #[test]
    fn test_command_construction() {
        let node = Sui::default()
            .with_force_regenesis(true)
            .with_faucet(true)
            .with_faucet_port(6123)
            .with_fullnode_rpc_port(8000)
            .with_epoch_duration_ms(60000);

        let cmd: Vec<String> = node
            .cmd()
            .into_iter()
            .map(|c| c.into().into_owned())
            .collect();

        assert_eq!(
            cmd,
            vec![
                "start",
                "--force-regenesis",
                "--with-faucet=6123",
                "--fullnode-rpc-port",
                "8000",
                "--epoch-duration-ms",
                "60000",
            ]
        );
        assert_eq!(node.entrypoint(), Some("sui"));
    }

    #[test]
    #[should_panic(expected = "with_network_config and with_force_regenesis are mutually exclusive")]
    fn test_mutually_exclusive_network_config_and_force_regenesis() {
        let _node = Sui::default()
            .with_force_regenesis(true)
            .with_network_config("/tmp/sui_config");
    }

    #[test]
    fn test_env_vars() {
        let node = Sui::default();
        let env: Vec<(String, String)> = node
            .env_vars()
            .into_iter()
            .map(|(k, v)| (k.into().into_owned(), v.into().into_owned()))
            .collect();
        assert_eq!(env.len(), 1);
        assert_eq!(
            env[0],
            ("RUST_LOG".to_string(), "warning,sui_node=info".to_string())
        );
    }

    #[test]
    fn test_expose_ports_with_faucet() {
        let node = Sui::default().with_faucet(true);
        let ports = node.expose_ports();
        assert_eq!(ports.len(), 2);
        // Verify that ports contain FULLNODE_RPC_PORT and FAUCET_PORT.
        assert_eq!(ports[0], FULLNODE_RPC_PORT);
        assert_eq!(ports[1], FAUCET_PORT);
    }

    #[test]
    fn test_expose_ports_without_faucet() {
        let node = Sui::default().with_faucet(false);
        let ports = node.expose_ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0], FULLNODE_RPC_PORT);
    }

    #[test]
    fn test_staging_tag() {
        let node = Sui::staging();
        assert_eq!(node.tag.as_deref(), Some("staging"));
    }

    #[test]#[test]
    fn test_ready_conditions() {
        let node = Sui::default();
        let conditions = node.ready_conditions();
        assert_eq!(conditions.len(), 1);

        let debug_str = format!("{:?}", conditions[0]);
        println!("Debug output for wait condition: {}", debug_str);

        assert!(debug_str.contains("method: POST"));
        assert!(debug_str.contains("path: \"/\""));
        assert!(debug_str.contains("content-type"));
        assert!(debug_str.contains("application/json"));
        let expected_body = r#"Some(b"{\"jsonrpc\":\"2.0\",\"method\":\"sui_getChainIdentifier\",\"id\":1}")"#;
        assert!(
            debug_str.contains(expected_body),
            "Expected debug output to contain: {}\nBut got: {}",
            expected_body,
            debug_str
        );
    }

    #[tokio::test]
    async fn test_container_running_http() -> Result<(), Box<dyn std::error::Error>> {
        use reqwest::header::{HeaderValue, CONTENT_TYPE};
        use std::time::Duration;

        // Configure the Sui container as needed.
        let sui = Sui::default()
            .with_force_regenesis(true)
            .with_faucet(true)
            .with_faucet_port(6123)
            .with_fullnode_rpc_port(9000)
            .with_epoch_duration_ms(60000);
        
        // Start the container.
        let container: testcontainers::ContainerAsync<Sui> = sui.start().await?;
        
        // Give the container a few seconds to get ready. In a real-world scenario,
        // the HttpWaitStrategy should ensure readiness, but we add a short delay here.
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // Retrieve the host port mapped for the internal FULLNODE_RPC_PORT.
        let host_port = container.get_host_port_ipv4(FULLNODE_RPC_PORT).await?;
        let url = format!("http://localhost:{}", host_port);

        // Create an HTTP client and send the POST request.
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(r#"{"jsonrpc":"2.0","method":"sui_getChainIdentifier","id":1}"#)
            .send()
            .await?;

        // Verify that we received a 200 OK response.
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        // Optionally, parse the JSON response.
        let json: serde_json::Value = response.json().await?;
        println!("Received JSON: {}", json);
        
        // For example, assert that there is a "result" field in the response.
        assert!(json.get("result").is_some(), "Expected a 'result' field in the JSON response");

        Ok(())
    }

}
