use std::env;

use testcontainers_modules::{
    sui::Sui,
    testcontainers::{runners::AsyncRunner, ImageExt},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    // env::set_var("RUST_LOG", "debug");
    let _ = pretty_env_logger::try_init();
    // Get the container request from our custom function.
    let sui_node_request = setup_sui_node();

    // Start the container, log any error and then panic.
    let container: testcontainers::ContainerAsync<_> = sui_node_request.start().await?;
    Ok(())
}

fn setup_sui_node() -> Sui {
    Sui::default()
        .with_force_regenesis(true)
        .with_faucet(true)
        .with_faucet_port(6123)
        .with_fullnode_rpc_port(9000)
        .with_epoch_duration_ms(60000)
}
