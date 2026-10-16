![](./Banner.svg)

# Description
  The software emulates the components of a ship using Linux Network Namespaces. The core security software, which is the appliacation layer firewall, is then attached to the components' network. Using policy TOML files a user is able to define communication rules for the network.

# Usage
  Use the Makefile to create the components and then edit the policy TOML files to define communication rules... have fun I guess!  

# Dependencies
Your system should have installed the following :
- [Rust Programming Language](https://www.rust-lang.org/learn/get-started)
- [Rust Nightly Toolchain](https://stackoverflow.com/questions/63348822/how-to-install-nightly)
- [Libbpf](https://github.com/libbpf/libbpf)
