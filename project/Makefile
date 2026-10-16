CONTAINERS := 1 2 3 4 5 6 7 8

# Simulation defaults, override on the command line, e.g.:
#   make sender TARGET=10.42.0.70:8888 DURATION=10 THREADS=2
POLICY ?= ./policies/policy_0.toml
NAMESPACE ?= test1
TARGET ?= 10.42.0.60:8888
DURATION ?= 30
THREADS ?= 4

.PHONY: build
build:
	cargo build --release --workspace

# Builds and runs the simulation as root (AF_XDP requires privileges).
# Run `make test-net` first: the component interfaces and namespaces must exist.
.PHONY: run
run: build
	sudo ./target/release/simulation $(POLICY)

# UDP load generator that sends valid NMEA sentences ($IIHDT) from a component namespace.
# It must run inside a namespace: the firewall intercepts frames sent by the components
# on the host side of their veth pairs.
.PHONY: sender
sender: build
	sudo ip netns exec $(NAMESPACE) ./target/release/sender $(TARGET) $(DURATION) $(THREADS)

.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: lint
lint:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings

.PHONY: test
test:
	cargo test --workspace

.PHONY: test-container-%
test-container-%:
	$(eval I := $(subst test-container-,,$@))

	sudo ip netns add test$(I)
	sudo ip netns exec test$(I) ip link set dev lo up
	sudo ip netns exec test$(I) ip link add eth0 type veth peer name test$(I)
	sudo ip netns exec test$(I) ip link set dev test$(I) netns 1
	sudo ip netns exec test$(I) ip link set dev eth0 address 54:00:00:00:00:$(I)0
	sudo ip netns exec test$(I) ip addr add 10.42.0.$(I)0/24 dev eth0
	sudo ip netns exec test$(I) ip link set dev eth0 up
	for j in $(CONTAINERS); do \
		sudo ip netns exec test$(I) ip neigh replace 10.42.0.$${j}0 lladdr 54:00:00:00:00:$${j}0 nud permanent dev eth0; \
	done
	sudo ip link set dev test$(I) up
	sudo ip link set dev test$(I) address 54:00:00:00:00:0$(I)
	sudo ip addr add 10.42.0.$(I)/32 dev test$(I) noprefixroute
	sudo ip route add to 10.42.0.$(I)0/32 dev test$(I) src 10.42.0.$(I) 
	sudo ethtool -K test$(I) tx off
	sudo ip netns exec test$(I) ethtool -K eth0 tx off

.PHONY: clean-container-%
clean-container-%:
	$(eval I := $(subst clean-container-,,$@))
	sudo ip netns del test$(I) 2>/dev/null || true
	sudo ip link del test$(I) 2>/dev/null || true
	
.PHONY: test-net
test-net: $(foreach container,$(CONTAINERS),test-container-$(container) )

.PHONY: clean
clean: $(foreach container,$(CONTAINERS),clean-container-$(container) )

.PHONY: shell-%
shell-%:
	sudo ip netns exec $* bash
