use crate::StableStorage;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Creates a new instance of stable storage.
pub async fn build_stable_storage(root_storage_dir: PathBuf) -> Box<dyn StableStorage> {
    Box::new(POSIXFileSystemStableStorage::new(root_storage_dir))
}

struct POSIXFileSystemStableStorage {
    dir: PathBuf,
}

impl POSIXFileSystemStableStorage {
    fn new(dir: PathBuf) -> Self {
        POSIXFileSystemStableStorage { dir }
    }
}

#[async_trait::async_trait]
impl StableStorage for POSIXFileSystemStableStorage {
    /// Stores `value` under `key`.
    ///
    /// Detailed requirements are specified in the description of the assignment.
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        if key.len() > 255 {
            return Err("key too long".to_string());
        }
        if value.len() > 65535 {
            return Err("value too long".to_string());
        }
        let mut hasher = Sha256::new();

        hasher.update(key);

        let result = hasher.finalize();

        let filename = base64::encode_config(result, base64::URL_SAFE_NO_PAD);

        let mut filepath = self.dir.clone();
        filepath.push(filename);

        let mut tmppath = self.dir.clone();
        tmppath.push("tmpfile");

        let mut tmpfile = File::create(&tmppath).await.unwrap();

        let contents = base64::encode(value);

        tmpfile.write_all(contents.as_ref()).await.unwrap();

        tmpfile.sync_data().await.unwrap();

        fs::rename(tmppath, filepath).await.unwrap();

        File::open(&self.dir)
            .await
            .unwrap()
            .sync_data()
            .await
            .unwrap();

        Ok(())
    }

    /// Retrieves value stored under `key`.
    ///
    /// Detailed requirements are specified in the description of the assignment.
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        if key.len() > 255 {
            return None;
        }
        // create a Sha256 object
        let mut hasher = Sha256::new();

        // write input message
        hasher.update(key);

        // read hash digest and consume hasher
        let result = hasher.finalize();

        let filename = base64::encode_config(result, base64::URL_SAFE_NO_PAD);

        let mut filepath = self.dir.clone();
        filepath.push(filename);

        let maybe_file = File::open(&filepath).await;

        let mut file = match maybe_file {
            Ok(file) => file,
            Err(error) => match error.kind() {
                tokio::io::ErrorKind::NotFound => {
                    return None;
                }
                other_error => {
                    panic!("Problem opening the file: {:?}", other_error);
                }
            },
        };

        let mut content = vec![];
        file.read_to_end(&mut content).await.unwrap();

        Some(content)
    }

    /// Removes `key` and the value stored under it.
    ///
    /// Detailed requirements are specified in the description of the assignment.
    async fn remove(&mut self, key: &str) -> bool {
        if key.len() > 255 {
            return false;
        }
        // create a Sha256 object
        let mut hasher = Sha256::new();

        // write input message
        hasher.update(key);

        // read hash digest and consume hasher
        let result = hasher.finalize();

        let filename = base64::encode_config(result, base64::URL_SAFE_NO_PAD);

        let mut filepath = self.dir.clone();
        filepath.push(filename);

        let result = fs::remove_file(filepath).await.is_ok();

        File::open(&self.dir)
            .await
            .unwrap()
            .sync_data()
            .await
            .unwrap();

        return result;
    }
}
