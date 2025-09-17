# DAWNLib

Game development library for developed alongside with
the [DAWN](https://github.com/Coestaris/dawn) pet-project.

This is not a fully-featured game engine, but rather a ready-to-use threading 
model, OS-dependent window/surface, audio processing abstractions,
resource management system, and a wrapper around the user's rendering pipeline.
It is designed to allow user to focus on graphics programming and game logic
without having to worry about low-level details.

#### Features

- ECS architecture;
- Manually defined rendering pipeline using custom-made passes;
- Manually defined audio pipeline processing using pre-defined effects and generators;
- Asynchronous resource loading and support for custom resource containers.
- Windows, Linux, and macOS support. WASM is partially supported.

Currently, only OpenGL is supported. There are no plans for creating a fancy
abstracted API for graphics APIs, since user is heavily encouraged to
use graphics API directly. Maybe in the future, I'll add support for Vulkan and Metal.

#### Key dependencies
- [evenio](https://crates.io/crates/evenio) - Event-based ECS implementation;
- [glow](https://crates.io/crates/glow)/[glutin](https://crates.io/crates/glutin)/[winit](https://crates.io/crates/winit) - OS-dependent Window/Context managing trio;
- [cpal](https://crates.io/crates/cpal) - OS-dependent audio output library; 
- and lots of other awesome crates.

#### Project Structure

- `crates/assets` - Contains implementation of Asset Hub, which is used to load and
  manage game assets. It also describes the internal representation of assets.[release.yml](../.github/workflows/release.yml)
- `crates/audio` - Contains audio processing pipelines and effects.
- `crates/ecs` - Define Main Loop and some common ECS components and events.
- `crates/graphics` - Contains OS-dependent Window management, input handling, and
  rendering pipelines on top of OpenGL.
- `crates/util` - Contains various utility functions and types used across the project.
- `crates/dac` - Contains general data types of Dawn Asset Container (DAC) file format 
  as well as implementation of DAC file reader / writer.
  User is not forced to use a DAC file format for assets, it's just one of the 
  possible options.
- `dacgen` - Implementation of a DAC file writer. It is responsible for converting 
  raw assets (like .png, .wav, .glb, etc.) into an IR (intermediate representation)
  format and storing them in a DAC file.
