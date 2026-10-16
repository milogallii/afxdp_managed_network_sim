![](./Banner.svg)

# AF_XDP Managed Network Simulator

Emulates the networked components of a ship using Linux network namespaces and AF_XDP sockets.
An application layer firewall inspects the traffic on every component interface and only lets
NMEA 0183 sentences through that are explicitly allowed by user defined policy TOML files.

## How it works

`make test-net` creates eight network namespaces (`test1`..`test8`, IPs `10.42.0.10`..`10.42.0.80`)
whose veth peers (`test1`..`test8`) live in the host namespace. Each peer gets an XDP program + an
AF_XDP socket owned by a `shipcomponent`, all driven by the `ship` event loop. The XDP program sits
on the ingress path of the host-side veth, so it intercepts the frames **sent by the components**;
traffic written to an AF_XDP TX ring is delivered back into the corresponding namespace.

- **sends**: a component may only transmit NMEA sentences whose prefix - talker id + sentence type,
  e.g. `$IIHDT` or `!AIVDM` - is listed in its policy. Non-NMEA traffic and sentences with an
  invalid checksum (`*XX`) are dropped (default deny).
- **receives**: an allowed sentence is multicast to every component that lists its prefix; the ship
  switch rewrites the destination MAC/IP of the forged frames.
- Traffic statistics are printed at the end of the run and plotted as PNG files.

## Cargo workspaces (in `project/`)

| Workspace        | Role |
| ---------------- | ---- |
| `simulation`     | Entry point: loads the policy, sets up the components and the ship, prints results |
| `ship`           | Event loop: polls the AF_XDP sockets, forwards allowed traffic, summarizes performance |
| `shipcomponent`  | One ship component: veth + AF_XDP socket, send policy enforcement, statistics |
| `policy_handler` | Parses and validates the policy TOML files |
| `packet_parser`  | Dissects Ethernet/IPv4/UDP frames (zero copy) |
| `nmea`           | NMEA 0183 parser with checksum validation |

## Dependencies

- Linux with AF_XDP support (kernel >= 4.18, >= 5.4 recommended) and root privileges
- [Rust](https://www.rust-lang.org/learn/get-started) stable (edition 2024, >= 1.85)
- libbpf build dependencies: `clang`, `libelf`, `zlib`, `pkg-config`
  (Debian/Ubuntu: `sudo apt install clang libelf-dev zlib1g-dev pkg-config`)

The AF_XDP bindings come from the [`xdrippi`](https://crates.io/crates/xdrippi) crate.

## Usage

```bash
cd project
make test-net   # create the veth/namespace topology (needs sudo)
make run        # build + run the simulation as root
```

While the simulation is running, generate traffic from the `girobussola` component (its policy
allows `$IIHDT`). The sender must run inside a component namespace - the firewall intercepts the
frames components send on the host side of their veth pairs:

```bash
make sender
# or directly: sudo ip netns exec test1 ./target/release/sender 10.42.0.60:8888 30 4

# the ais component (test2) only allows !AIVDM/!AIVDO, so its frames are dropped:
sudo ip netns exec test2 ./target/release/sender 10.42.0.60:8888 30 4
```

At the end of the 30 s monitoring window the per-component summary is printed and the performance
plots are written to `project/test/imgs/`. Tear the topology down with `make clean`.

## Policies

Policies live in `project/policies/*.toml` and describe every component and the NMEA sentences it
may send and receive:

```toml
[policy]
c1 = { name = "ais", iface = "test2", mac = "54:00:00:00:00:20", ip = "10.42.0.20", sends = [
  "!AIVDM",
  "!AIVDO",
], receives = [] }
```

Prefixes are matched exactly and include the leading `$` (talker sentences) or `!` (encapsulated
sentences, e.g. AIS). `make run POLICY=./policies/other.toml` runs a different policy file.

## Development

```bash
make fmt    # rustfmt
make lint   # fmt check + clippy (warnings are errors)
make test   # unit tests (no root required)
```

CI runs formatting, clippy, tests and a release build on every push
(`.github/workflows/ci.yml`).

## Notes and limitations

- Frames are inspected individually, there is no IP fragment reassembly: keep test datagrams below
  the MTU (the bundled sender uses a 1400 byte payload for this reason).
- Only UDP/IPv4 NMEA traffic is modeled; every other protocol or payload is denied by default.
  `make test-net` installs permanent neighbor entries in every namespace, so components never need
  ARP (ARP frames themselves are also dropped by the default deny firewall).
- Traffic sent by the host stack directly to a component IP is not intercepted (XDP only sees the
  ingress path of the veths).
- The monitoring window is fixed at 30 s (`MONITOR_WINDOW` in `project/ship/src/lib.rs`).
