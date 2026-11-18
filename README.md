# NeuroFix

NeuroFix is a desktop utility for Windows that manages the DNS resolvers used by the system network adapter. It provides a single interface for browsing resolvers, measuring their response time, checking whether the most common AI services are reachable from the local network, and applying a chosen resolver to the system with one action.

Current version: **1.3.0**

---

## Overview

The application is written in Rust and uses a native GPU-rendered interface. The visual language follows Material You (Material 3): tonal surfaces, rounded containers, a navigation rail and state layers.

NeuroFix does not modify traffic routing and does not install drivers, services or background agents. The only system change it performs is setting the DNS servers of the active network adapter, and only after an explicit user action and the Windows elevation prompt.

## Features

### Resolver library
- A catalog of resolvers with their primary and secondary IPv4 addresses.
- Instant search by resolver name or IP address.
- Custom resolvers can be added by the user and are stored locally.

### Latency measurement
- Each resolver is queried directly over UDP port 53.
- The displayed value is the median response time of several requests, measured on the local machine.
- The **Auto select** action measures the whole catalog and selects the resolver with the lowest latency.

### Service availability
Each resolver card reports whether Gemini, Claude and ChatGPT are reachable through the current network when that resolver is in use. The check is performed locally and consists of two steps:

1. the service host name is resolved through the resolver being tested;
2. a TCP connection is established to the resolved address on port 443.

A service is reported as available only if both steps succeed. The state is a measurement of the current network conditions and can change when the network changes.

### Applying and reverting
- Applying a resolver writes its addresses to the active adapter.
- Reverting restores the automatic (DHCP) DNS configuration.
- Both operations require administrator rights; Windows displays the elevation prompt.

### Automatic recovery
When a resolver has been applied by the application and name resolution through it starts failing, the application:

1. verifies general network connectivity;
2. switches to the next healthy resolver from the catalog;
3. reverts to the automatic DNS configuration if no resolver is reachable.

This behaviour can be disabled in the interface.

## Requirements

- Windows 10 or Windows 11
- Administrator rights for changing DNS configuration
- An active network connection

## Installation

Download the latest release archive, extract it and run `NeuroFix.exe`. The application is a single executable and does not require a separate runtime.

## Usage

1. Open the **DNS** section.
2. Use the search field to filter the list, or press **Auto select** to measure every resolver and apply the fastest one.
3. Press **Apply** on a resolver card to set it for the active adapter.
4. Press **Reset** to return to the automatic DNS configuration.

The **Home** section shows the active adapter, the DNS servers currently in use and the application status. The **VPN** section is not available yet. The **About** section displays version information.

## How the checks work

| Check | Method | Reported as |
| --- | --- | --- |
| Latency | UDP DNS query to the resolver, median of the sample | milliseconds |
| Resolution | UDP DNS query for the service host name | success or failure |
| Reachability | TCP connection to the resolved address on port 443 | success or failure |
| Service state | Resolution and reachability combined | available or unavailable |
| Connectivity | DNS query for a control host name | online or offline |

All measurements are produced on the client machine. No telemetry is collected and no data leaves the device.

## Project structure

```
src/
  main.rs        application entry point and window configuration
  app.rs         application state and page composition
  theme.rs       Material 3 color tokens, typography and spacing
  icons.rs       vector icons drawn at runtime
  widgets.rs     reusable interface elements
  dns.rs         resolver catalog and the DNS query client
  checks.rs      background workers for latency and availability
  sysdns.rs      reading and writing the adapter DNS configuration
```

## Building from source

A Rust toolchain is required. On Windows either the MSVC toolchain with the C++ workload of Visual Studio Build Tools, or the GNU toolchain with a MinGW-w64 distribution, is supported.

```
cargo build --release
```

With the GNU toolchain:

```
cargo +stable-x86_64-pc-windows-gnu build --release
```

The executable is written to `target/release/neurofix.exe`.

Run the checks before submitting changes:

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

## Contributing

Bug reports and feature requests are accepted through the issue templates in `.github/ISSUE_TEMPLATE`. See `CONTRIBUTING.md` for details.

## License

Distributed under the Apache License 2.0. See `LICENSE` for the full text.
