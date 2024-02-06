# jojo-server

This repository hosts the WebSocket server component of the [jojo]((https://github.com/gggiulio77/jojo)) project. It operates on the host computer, where control over PC peripherals is desired. While it can function independently, the primary intention is to run it alongside the [jojo-app](https://github.com/gggiulio77/jojo-app) as a separate process, facilitating event-based communication between the two.

### Quick Links

- [Getting Started](#getting-started)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [Roadmap](#roadmap)
- [License](#license)

## Getting Started

This server have some responsibilities:
- Running a WebSocket server to receive messages from [jojo-client](https://github.com/gggiulio77/jojo-client).
- Initializing all necessary drivers for managing peripherals on the host PC (keyboard, mouse and "virtual" gamepad).
- Storing all connected devices in a database.
- Implementing a full-duplex channel with [jojo-app](https://github.com/gggiulio77/jojo-app).

### Prerequisites

This server currently runs only on Windows (for now...).

Before proceeding, ensure you have [Rust](https://www.rust-lang.org/tools/install) installed on your system.

To control a gamepad, a virtual joystick is required. Currently, Vjoy is a necessary dependency. You can download it from [here](https://sourceforge.net/projects/vjoystick/files/Beta%202.x/2.1.9.1-160719/). The server will attempt to acquire a device from Vjoy, so ensure you have at least one available.

### Installation

`git clone https://github.com/gggiulio77/jojo-server.git`

## Usage

To execute the project as a binary, utilize the `cargo run` command. The main.rs file serves as the entry point. Within this file, parameters are hardcoded to invoke the `initialize` function located inside lib.rs. This setup mirrors how the [jojo-app](https://github.com/gggiulio77/jojo-app) will initialize this server in a different process.

Once up, you can connect with the server through `/ws` endpoint with a uuid as a path param. You can find an [insomnia](https://insomnia.rest/) project to test it. 

## Roadmap

- [ ] Implement feature flags to manage which driver is going to run
- [ ] Improve error handling
- [ ] Make the server cross platform (Linux, Linux(arm), MacOs)

## License
