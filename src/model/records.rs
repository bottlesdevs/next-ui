//! Files the application owns, each written in the order it was changed.

use std::{
    io,
    ops::Deref,
    path::{Path, PathBuf},
};

use iced::futures::{FutureExt as _, channel::oneshot, future::BoxFuture};
use next_config::Config;

/// Loads the file at `path`, or the default value when it does not exist.
pub(crate) async fn load<T: Config + Default>(path: &Path) -> Result<T, String> {
    match next_config::load(path).await {
        Ok(value) => Ok(value),
        Err(next_config::error::Error::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            Ok(T::default())
        }
        Err(error) => Err(format!("failed to load {}: {error}", path.display())),
    }
}

/// A value and the file it is saved to.
pub(crate) struct Saved<T> {
    value: T,
    path: PathBuf,
    tail: Option<oneshot::Receiver<()>>,
}

impl<T> Deref for Saved<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: Config + Clone + Send + Sync + 'static> Saved<T> {
    pub(crate) fn new(value: T, path: PathBuf) -> Self {
        Self {
            value,
            path,
            tail: None,
        }
    }

    /// Applies `change` and returns the write that saves the result.
    ///
    /// Writes of one file finish in the order their changes were made.
    pub(crate) fn change(
        &mut self,
        change: impl FnOnce(&mut T),
    ) -> BoxFuture<'static, Result<(), String>> {
        change(&mut self.value);
        let (completion, next) = oneshot::channel();
        let previous = self.tail.replace(next);
        let (value, path) = (self.value.clone(), self.path.clone());
        async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            let result = next_config::save(&path, &value)
                .await
                .map_err(|error| format!("failed to save {}: {error}", path.display()));
            drop(completion);
            result
        }
        .boxed()
    }
}
