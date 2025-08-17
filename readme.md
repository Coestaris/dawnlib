# DAWNLib

Modular game engine, developed alongside
the [DAWN](https://github.com/Coestaris/dawn) pet-project.

#### Features

- ECS architecture using [evenio](https://github.com/rj00a/evenio) library.
- Customizable rendering pipeline.
- OpenGL 4.2+ support.
- Windows, Linux, and macOS support.
- Real-time audio processing pipelines, allowing for applying audio effects and
  mixing.
- Asynchronous resource loading and support for custom resource containers.

### Project Structure

- `assets` - Contains implementation of Asset Hub, which is used to load and
  manage game assets. It also describes the internal representation of assets.
- `audio` - Contains audio processing pipelines and effects.
- `ecs` - Define Main Loop and some common ECS components and events.
- `graphics` - Contains OS-dependent Window management, input handling, and
  rendering pipelines on top of OpenGL.
- `profile` - Contains profiling tools and utilities.
- `yarc` - Contains an implementation of Asset container. It also provides 
  utilities for converting raw assets (like .png, .wav, etc.) into
  internal representation of assets.

#### Prerequisites

You need to have installed `libx11-dev` and `libdbus-1-dev` when building on
Linux.
Other dependencies are handled by Cargo.


