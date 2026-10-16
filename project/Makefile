CONTAINERS := 1 2 3 4 5 6 7 8

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
	sudo ip netns exec test$(I) ip neigh add 10.42.0.$(I)0 lladdr 54:00:00:00:00:$(I)0 nud permanent dev eth0
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
	$(eval NAME := $(subst shell-,,$@))
	sudo ip netns exec $(NAME) bash
