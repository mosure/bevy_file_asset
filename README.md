# bevy_file_asset 🧩

[![crates.io](https://img.shields.io/crates/v/bevy_file_asset.svg)](https://crates.io/crates/bevy_file_asset)

bevy asset loader supporting files outside of the asset folder


## minimal example

```rust
use bevy::prelude::*;
use bevy_file_asset::FileAssetPlugin;

fn main() {
    App::new()
        .add_plugins((
            FileAssetPlugin,
            DefaultPlugins,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d::default());

    let image = asset_server.load("file://docs/image.png");
    commands.spawn(Sprite {
        image,
        ..Default::default()
    });
}
```


## compatible bevy versions

| `bevy_args` | `bevy` |
| :--         | :--    |
| `0.3`       | `0.17` |
| `0.2`       | `0.16` |
| `0.1`       | `0.15` |
