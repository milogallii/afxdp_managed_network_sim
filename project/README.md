# PROJECT STRUCTURE

## Cargo Workspaces

| Workspace        | Role |
| --------         | ------- |
| simulation       | Emulation of a ship whose components networking is monitored with afxdp|
| ship             | Emulates a ship with higher level functions ( monitoring, firewall, ...) |
| ship_component    | Emulates a ship component with its own veth and an afxdp socket bound to it  |
| policy_handler| Handles policy files containing rules for ship components' networking | 
| packet_parser | Parses network packets and analyses them |
| nmea | Parser for nmea sentences | 
 

## Additional files

| File     | Utility |
| -------- | ------- |
| Makefile | Sets up the Linux namespaces for ship's components' veths |
| Policies | Directory containing networking policies |


