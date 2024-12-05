use crate::solution::stable_storage::build_stable_storage;
use crate::*;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::File;

pub(crate) struct PathsManager {
    root_path: PathBuf,
    sectors_manager: Option<Arc<dyn SectorsManager>>,
    set: HashSet<u8>,
}

impl PathsManager {
    pub(crate) async fn new(root_path: PathBuf) -> Self {
        Self {
            root_path,
            sectors_manager: None,
            set: HashSet::new(),
        }
    }

    pub(crate) async fn get_stable_storage(&mut self, worker_id: u8) -> Box<dyn StableStorage> {
        assert!(!self.set.contains(&worker_id));
        self.set.insert(worker_id);
        let mut dir = self.root_path.clone();
        dir.push("stable_storage");
        fs::create_dir_all(dir.clone())
            .await
            .expect("Couldn't create stable storage dir");
        File::open(&self.root_path)
            .await
            .expect("failed to open rooth path")
            .sync_data()
            .await
            .expect("failed to sync data");
        let storage_dir = dir.clone();
        dir.push(format!("worker_{:#04x}", worker_id));
        fs::create_dir_all(dir.clone())
            .await
            .expect("couldn't create worker dir");
        File::open(storage_dir)
            .await
            .expect("failed to open storage dir")
            .sync_data()
            .await
            .unwrap();
        File::open(&dir).await.unwrap().sync_data().await.unwrap();
        build_stable_storage(dir).await
    }

    pub(crate) async fn get_sectors_manager(&mut self) -> Arc<dyn SectorsManager> {
        if self.sectors_manager.is_none() {
            let mut dir = self.root_path.clone();
            dir.push("sectors_manager");
            fs::create_dir_all(dir.clone()).await.unwrap();
            File::open(&self.root_path)
                .await
                .unwrap()
                .sync_data()
                .await
                .unwrap();
            File::open(&dir).await.unwrap().sync_data().await.unwrap();
            self.sectors_manager = Some(build_sectors_manager(dir).await);
        }
        self.sectors_manager.as_ref().unwrap().clone()
    }
}
