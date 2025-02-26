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
