#![warn(missing_docs)]
//! # File Asset Plugin for Bevy
//!
//! This plugin registers an asset source under the `"file"` scheme which loads assets
//! directly from arbitrary paths (both relative to the working directory and absolute).
//!
//! If the file does not exist at the given path, the reader returns a `NotFound` error,
//! so that Bevy’s asset server can fall back to other methods.
//!
//! ## Example Usage
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_file_asset::FileAssetPlugin;
//!
//! let mut app = App::new();
//! // Register the file asset source before adding the DefaultPlugins.
//! app.add_plugins((FileAssetPlugin::default(), DefaultPlugins));
//! ```

use bevy::{
    asset::io::{AssetReader, AssetReaderError, AssetSource, PathStream, Reader, VecReader},
    prelude::*,
    utils::ConditionalSendFuture,
};
use futures::{
    channel::oneshot,
    stream::{self, Stream},
};
use std::{
    fs,
    path::{Path, PathBuf},
    pin::Pin,
    thread,
};

/// Spawns a blocking operation on a dedicated thread and awaits its result.
async fn spawn_blocking<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    thread::spawn(move || {
        let res = f();
        let _ = tx.send(res);
    });
    rx.await.expect("spawn_blocking thread panicked")
}

/// A reader that loads files directly from arbitrary paths.
pub struct FileAssetReader;

impl FileAssetReader {
    /// Asynchronously reads the file at the given path using blocking I/O on a dedicated thread.
    async fn file_get(path: PathBuf) -> Result<Box<dyn Reader>, AssetReaderError> {
        let path_for_error = path.clone();
        let result = spawn_blocking(move || fs::read(&path)).await;

        match result {
            Ok(bytes) => Ok(Box::new(VecReader::new(bytes))),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Err(AssetReaderError::NotFound(path_for_error))
                } else {
                    Err(AssetReaderError::Io(err.into()))
                }
            }
        }
    }

    /// Constructs a meta file path by appending ".meta" to the file’s extension.
    ///
    /// For example, given "image.png" it returns "image.png.meta".
    fn make_meta_path(path: &Path) -> Option<PathBuf> {
        let ext = path.extension()?;
        let mut meta_ext = ext.to_os_string();
        meta_ext.push(".meta");
        let mut meta_path = path.to_path_buf();
        meta_path.set_extension(meta_ext);
        Some(meta_path)
    }
}

impl AssetReader for FileAssetReader {
    #[allow(refining_impl_trait)]
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<dyn Reader>, AssetReaderError>> + 'a {
        let path_buf = path.to_path_buf();
        async move {
            // Attempt to load the file directly from the given path.
            if path_buf.exists() {
                Self::file_get(path_buf).await
            } else {
                // If the file isn’t found, return a NotFound error to signal fallback.
                Err(AssetReaderError::NotFound(path.to_path_buf()))
            }
        }
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<dyn Reader>, AssetReaderError> {
        if let Some(meta_path) = Self::make_meta_path(path) {
            if meta_path.exists() {
                Self::file_get(meta_path).await
            } else {
                Err(AssetReaderError::NotFound(meta_path))
            }
        } else {
            Err(AssetReaderError::NotFound("source path has no extension".into()))
        }
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(path.is_dir())
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        if path.is_dir() {
            let entries = fs::read_dir(path)
                .map_err(|e| AssetReaderError::Io(e.into()))?
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .collect::<Vec<_>>();
            // Create a stream from the vector of paths.
            let stream = stream::iter(entries);
            // Box and pin the stream to satisfy the PathStream type.
            let boxed_stream: Pin<Box<dyn Stream<Item = PathBuf> + Send>> = Box::pin(stream);
            Ok(Box::new(boxed_stream))
        } else {
            Err(AssetReaderError::NotFound(path.to_path_buf()))
        }
    }
}

/// Plugin that registers the file asset source.
///
/// This plugin enables loading assets from arbitrary file paths (relative or absolute)
/// by registering a new asset source under the `"file"` scheme.
#[derive(Default)]
pub struct FileAssetPlugin;

impl Plugin for FileAssetPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            "file",
            AssetSource::build().with_reader(|| Box::new(FileAssetReader)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_asset_reader() {
        // Create a temporary file with known contents.
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, Bevy!").unwrap();
        let path = file.path().to_path_buf();

        let reader = FileAssetReader.read(&path).await.unwrap();
        let mut vec_reader = reader;
        let mut content = Vec::new();
        // Await the future returned by read_to_end
        vec_reader
            .read_to_end(&mut content)
            .await
            .expect("Failed to read content");
        assert!(String::from_utf8_lossy(&content).contains("Hello, Bevy!"));
    }

    #[tokio::test]
    async fn test_file_asset_reader_not_found() {
        let path = PathBuf::from("non_existent_file.txt");
        let result = FileAssetReader.read(&path).await;
        assert!(matches!(result, Err(AssetReaderError::NotFound(_))));
    }
}
