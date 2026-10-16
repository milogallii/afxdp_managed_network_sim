#!/bin/bash

# Create namespaces
sudo ip netns add test1
sudo ip netns add test2

# Create veth pairs and configure test1 namespace
sudo ip netns exec test1 ip link add eth0 type veth peer name test1
sudo ip netns exec test1 ip link set dev lo up
sudo ip netns exec test1 ip link set eth0 address 54:00:00:00:00:10
sudo ip netns exec test1 ip addr add 10.42.0.10/24 dev eth0
sudo ip netns exec test1 ip link set eth0 up

# Move veth peer to main namespace and disable TX checksum offload
sudo ip netns exec test1 ip link set test1 netns 1
sudo ip link set dev test1 up
sudo ethtool -K test1 tx off

# Configure test2 namespace
sudo ip netns exec test2 ip link add eth0 type veth peer name test2
sudo ip netns exec test2 ip link set dev lo up
sudo ip netns exec test2 ip link set eth0 address 54:00:00:00:00:20
sudo ip netns exec test2 ip addr add 10.42.0.20/24 dev eth0
sudo ip netns exec test2 ip link set eth0 up

# Move veth peer to main namespace and disable TX checksum offload
sudo ip netns exec test2 ip link set test2 netns 1
sudo ip link set dev test2 up
sudo ethtool -K test2 tx off

# Create bridge and attach interfaces
sudo ip link add name br0 type bridge
sudo ip link set br0 up
sudo ip link set test1 master br0
sudo ip link set test2 master br0

# Assign IP to bridge (optional for L3 communication)
sudo ip addr add 10.42.0.1/24 dev br0

# Disable TX checksum offload in namespaces
sudo ip netns exec test1 ethtool -K eth0 tx off
sudo ip netns exec test2 ethtool -K eth0 tx off

# Optional: Verify configuration
echo "=== test1 configuration ==="
sudo ip netns exec test1 ip -4 a show eth0
sudo ip netns exec test1 ip route

echo "=== test2 configuration ==="
sudo ip netns exec test2 ip -4 a show eth0
sudo ip netns exec test2 ip route

echo "=== Bridge configuration ==="
sudo brctl show br0
sudo ip -4 a show br0

# Test connectivity
# echo "Starting iperf3 server in test2..."
sudo ip netns exec test2 iperf3 -s &
  
# sleep 2  # Give server time to start
# echo "Starting iperf3 client in test1..."
sudo ip netns exec test1 iperf3 -c 10.42.0.20 -u -b 1G -l 1460

# Kill background server
pkill -f "iperf3 -s"

# Cleanup existing resources
sudo ip netns del test1 2>/dev/null
sudo ip netns del test2 2>/dev/null
sudo ip link del test1 2>/dev/null
sudo ip link del test2 2>/dev/null
sudo ip link del br0 2>/dev/null

