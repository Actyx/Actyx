# Actyx on Windows

This is a standalone crate and not within `node`, as there's a limitation in
Cargo, where adding resources (in this case the icon) to a binary is only
possible for a single binary target, not multiple ones. See `./build.rs`.