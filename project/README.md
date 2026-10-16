# PROJECT STRUCTURE

## Cargo Workspaces

| Workspace        | Role |
| --------         | ------- |
| simulation       | Entry point: loads the policy, builds the ship components and the ship, prints results |
| ship             | Event loop: polls the AF_XDP sockets, forwards allowed traffic, summarizes performance |
| shipcomponent    | Emulates a ship component with its own veth and an afxdp socket bound to it (send policy enforcement, stats) |
| policy_handler   | Loads and validates policy files containing rules for ship components' networking |
| packet_parser    | Parses network packets (Ethernet/IPv4/UDP payload, zero copy) |
| nmea             | Parser for NMEA sentences (checksum validation, talker id + sentence type) |

## Policy files

Each component entry in a policy file (`policies/*.toml`) defines:

- `sends`: NMEA prefixes (`$TTSSS` / `!TTSSS`) the component may transmit. Anything else -
  non-NMEA traffic, invalid checksums or unlisted prefixes - is dropped (default deny).
- `receives`: NMEA prefixes delivered to the component. Every allowed sentence is multicast to all
  components that list its prefix.

## Additional files

| File     | Utility |
| -------- | ------- |
| Makefile | Sets up the Linux namespaces for the components' veths; `make run` starts the simulation (as root), `make sender` generates test traffic, `make lint` / `make test` check the code |
| policies | Directory containing networking policies |
| test/imgs | Output directory of the performance plots |
| test/sender | UDP load generator emitting valid NMEA sentences below the MTU |
| test/linux_baseline.sh | Two-namespace + bridge baseline used for comparison experiments |
