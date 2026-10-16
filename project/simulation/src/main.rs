use policy_handler::PolicyHandler;
use ship::Ship;
use shipcomponent::ShipComponent;
fn main() {
    // Getting policy parameters for components
    let policy = PolicyHandler::new(String::from("./policies/policy_0.toml"));

    // Setting up ship components accordigly
    let mut ship_components: Vec<ShipComponent> = Vec::new();
    let policy = policy.get_policy();
    policy.iter().for_each(|component| {
        ship_components.push(ShipComponent::new(
            component.name.clone(),
            component.iface.clone(),
            component.mac.clone(),
            component.ip.clone(),
            component.sends.clone(),
            component.receives.clone(),
        ));
    });

    // Setting up ship
    let mut ship = Ship::new(ship_components);
    println!("STARTING SIMULATION");
    ship.monitor_network();

    println!("----------------------------------");
    ship.components.iter().for_each(|component| {
        println!(
            "[{}] - [TOTAL SENT: {:.2}Mb] [TOTAL RECEIVED: {:.2}Mb] [BITRATE SEND: {:.2}Mbit/s] [BITRATE RECEIVE: {:.2}Mbit/s] [TOTAL ANALYSIS TIME: {:.2}]",
            component.name,
            component.stats.total_bytes_sent / 1000000.0,
            component.stats.total_bytes_received / 1000000.0,
            (component.stats.total_bytes_sent * 8.0
                / component.stats.performance_send[component.stats.performance_send.len() - 1].0)
                / 1000000.0,
                            (component.stats.total_bytes_received * 8.0
                / component.stats.performance_receive[component.stats.performance_receive.len() - 1].0)
                / 1000000.0,
                component.stats.performance_send[component.stats.performance_send.len() - 1].0

        );
    });

    ship.components.iter().for_each(|component| {
        let _ = component
            .stats
            .plot_performance(&format!("{}-{}", component.ifname, component.name));
    });
}
