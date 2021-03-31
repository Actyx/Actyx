# Integration testing host platforms

| key | value |
| --- | --- |
| date | 2020-03-30 |
| status | accepted |
| persons | @mhaushofer, @rkuhn |

## Decision

We will cover the following Actyx & Actyx CLI host platforms in our integration tests for Actyx v2:

- Windows 10 Enterprise (the most restrictive of the Windows 10 variants, also used on rugged tablets)
- Windows Server 2019 (update from 2016 should be easily possible)
- Ubuntu 20.04
- macOS Big Sur
- Android 8.1 (no Actyx CLI)
- Docker 20 (no Actyx CLI)

These systems are tested on the following CPU architecture:

- Linux/Docker: x86_64, aarch64, armv7, arm
- Windows/macOS: x86_64
- Android: x86, aarch64, armv7

## Business context

Market research and partner inquiries indicate that the given Windows versions are the most important deployment target, followed by Linux.
Since we build statically linked binaries for Linux, we simplify the testing procedure to just one OS variant.
macOS is relevant for development, not only within Actyx.
Android is relevant for mobile applications, especially on the phone form factor.

iOS is out of scope for now, also due to insignificant usage on the factory shop-floor.
ARM builds for Windows or macOS are currently left aside since these platforms are not yet technically established well enough in the Rust ecosystem — we cannot afford to innovate on this front.
In case of Windows it would currently also not be worth it since the only available hardware with ARM processors are high-end notebook computers.

_The last two points are likely to change in the future, at which point we will revisit this decision._

## Consequences

- suitable binaries need to be built during CI (requires private Docker image for building macOS binaries — their SDK has copyright restrictions)
- suitable EC2 system images and host machines need to be identified (these exist for everything but Android) and integrated
- the OS/arch matrix specified above has 14 entries, leading to correspondingly complex test setup
- macOS on Apple Silicon will require Rosetta to run Actyx
- Windows on ARM is not supported at this time

The full list of OS/Arch strings we need to build native code for is thus (for corresponding Rust targets see top-level `Makefile`):

- linux-x86_64
- linux-aarch64
- linux-armv7
- linux-arm
- windows-x86_64
- macos-x86_64
