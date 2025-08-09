use async_zip::{
    base::read::stream::{Ready, ZipFileReader},
    error::ZipError,
};
use futures::io::{AsyncBufRead, AsyncWriteExt as _};
use tokio_util::compat::TokioAsyncWriteCompatExt as _;

use std::path::{Path, PathBuf};

/// Extracts an archive of [`ZipFileReader<Ready<R>>`] to a specified path.
/// This function will sanitize the file path and create intermediate directories if possible.
///
/// # Panics
///
/// Panics if the archive reader is [`None`], which is unreachable.
///
/// # Errors
///
/// Returns a [`ZipError`] if the archive fails to extract.
pub async fn extract_archive<R, P>(
    archive: ZipFileReader<Ready<R>>,
    path: P,
) -> Result<(), ZipError>
where
    R: AsyncBufRead + Unpin,
    P: AsRef<Path>,
{
    drop(tokio::fs::remove_dir_all(&path).await);
    tokio::fs::create_dir(&path).await?;

    let mut a_ready = Some(archive);

    fn sanitize_file_path(path: &str) -> PathBuf {
        // Replaces backwards slashes
        path.replace('\\', "/")
            // Sanitizes each component
            .split('/')
            .map(sanitize_filename::sanitize)
            .collect()
    }

    while let Some(mut a_reading) = a_ready
        .take()
        .expect("unreachable")
        .next_with_entry()
        .await?
    {
        let reader = a_reading.reader();
        let Ok(name) = reader.entry().filename().as_str() else {
            a_ready = Some(a_reading.skip().await?);
            continue;
        };
        let p = path.as_ref().join(sanitize_file_path(name));

        if name.ends_with('/') {
            // Is a directory
            if !p.exists() {
                tokio::fs::create_dir_all(&p).await?;
            }
        } else {
            // Creates parent directories. They may not exist if iteration is out of order or the archive does not contain directory entries
            let parent = p.parent().expect("cant be a root dir");
            if !parent.is_dir() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let mut writer = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&p)
                .await?
                .compat_write();
            futures::io::copy(a_reading.reader_mut(), &mut writer).await?;
            writer.flush().await?;
        }

        a_ready = Some(a_reading.done().await?);
    }

    Ok(())
}
