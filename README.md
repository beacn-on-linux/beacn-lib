# Beacn Device Protocol

This is a cross-platform library for communicating with Beacn Devices

***
### USE AT YOUR OWN RISK
This code directly modifies the on-board storage of Beacn devices. While it's been tested and made to be as safe as
is possible, it was derived from reverse engineering and thus may not be accurate.

This project is not supported by, or affiliated in any way with Beacn. For official Beacn software, please refer
to their website.

In addition, this project accepts no responsibility or liability for any use of this software, or any problems
which may occur from its use. Please read the LICENSE for more information.

***

## Features
- **Hot-plug Support**: Automatic device detection
- **Audio Devices**: Control all settings in the Beacn Mic and Studio
- **Control Devices**: Mix / Mix Create screen and interaction handling
- **Async-First**: Full async / await support, with optional .wait() for blocking calls

## Supported Devices
- Beacn Mic
- Beacn Studio
- Beacn Mix
- Beacn Mix Create
 
## Usage
```toml
[dependencies]
beacn-lib = { git = "https://github.com/beacn-on-linux/beacn-lib", tag = "v0.2.0", features = [] }
```
If you're using `tokio` or `smol` they should be included as features otherwise tasks such as listing and
opening devices will fall back to regular blocking calls.

## Warnings
When used against a Beacn Mix and Mix Create, termination of an application while an image is mid-send may cause
the firmware to lock up and require a power cycle. Ensure your apps have proper and clean shutdown handling to ensure
the connection to the device is closed.

#### Examples
The [`examples/`](examples) directory contains a number of examples which demonstrate all of the features
and concepts of this library, provided in both sync and async flavours.


