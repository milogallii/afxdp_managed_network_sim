use anyhow::{Context, Result};
use policy_handler::PolicyHandler;
use ship::Ship;
use shipcomponent::ShipComponent;

const DEFAULT_POLICY_PATH: &str = "./policies/policy_0.toml";

/// Runs the ship simulation for a fixed monitoring window.
///
/// Usage: simulation [POLICY_FILE]
fn main() -> Result<()> {
    let policy_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from(DEFAULT_POLICY_PATH));

    // Getting policy parameters for components
    let policy_handler = PolicyHandler::new(&policy_path)?;
    policy_handler.show_policy();

    // Setting up ship components accordingly
    let components = policy_handler
        .get_policy()
        .into_iter()
        .map(|component| {
            ShipComponent::new(
                component.name,
                component.iface,
                component.mac,
                component.ip,
                component.sends,
                component.receives,
            )
        })
        .collect::<Result<Vec<_>>>()
        .context("failed to set up one or more ship components")?;

    // Setting up ship
    let mut ship = Ship::new(components)?;
    println!("STARTING SIMULATION");
    ship.monitor_network()?;

    println!("----------------------------------");
    ship.components.iter().for_each(print_component_summary);

    ship.components.iter().for_each(|component| {
        let plot_name = format!("{}-{}", component.ifname, component.name);
        if let Err(error) = component.stats.plot_performance(&plot_name) {
            eprintln!("failed to plot stats for [{plot_name}]: {error}");
        }
    });

    Ok(())
}

fn print_component_summary(component: &ShipComponent) {
    let stats = &component.stats;
    let elapsed = stats.elapsed();

    let (send_bitrate, receive_bitrate) = if elapsed > 0.0 {
        (
            (stats.total_bytes_sent * 8.0 / elapsed) / 1_000_000.0,
            (stats.total_bytes_received * 8.0 / elapsed) / 1_000_000.0,
        )
    } else {
        (0.0, 0.0)
    };

    println!(
        "[{}] - [TOTAL SENT: {:.2}Mb] [TOTAL RECEIVED: {:.2}Mb] [BITRATE SEND: {:.2}Mbit/s] [BITRATE RECEIVE: {:.2}Mbit/s] [DROPPED FRAMES: {}] [TOTAL ANALYSIS TIME: {:.2}s]",
        component.name,
        stats.total_bytes_sent / 1_000_000.0,
        stats.total_bytes_received / 1_000_000.0,
        send_bitrate,
        receive_bitrate,
        stats.dropped_frames,
        elapsed
    );
}
