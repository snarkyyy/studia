use crate::solution::transfer::SECTOR_LEN;
use crate::{SectorIdx, SectorVec, SectorsManager};
use std::collections::HashMap;
use std::fs::DirEntry;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

use crate::solution::running::NUMBER_OF_WORKERS;

pub async fn build_sectors_manager(path: PathBuf) -> Arc<dyn SectorsManager> {
    Arc::new(FileSystemSectorsManager::new(path).await)
}

struct FileSystemSectorsManager {
    path: PathBuf,
    idx_to_meta: Vec<RwLock<HashMap<SectorIdx, (u64, u8)>>>,
}

fn encode_filename(data: (u64, u64, u8)) -> String {
    let mut buf = [0; 18];
    let mut buf_slice = &mut buf[..];
    let (sector_idx, logical_timestamp, write_rank) = data;
    io::Write::write_all(&mut buf_slice, &sector_idx.to_le_bytes()).unwrap();
    io::Write::write_all(&mut buf_slice, &logical_timestamp.to_le_bytes()).unwrap();
    io::Write::write_all(&mut buf_slice, &write_rank.to_le_bytes()).unwrap();
    let filename = base64::encode_config(buf, base64::URL_SAFE_NO_PAD);
    filename
}

fn decode_filename(filename: String) -> Result<(u64, u64, u8), base64::DecodeError> {
    let buf = base64::decode_config(filename, base64::URL_SAFE_NO_PAD)?;
    if buf.len() != 18 {
        return Err(base64::DecodeError::InvalidLength);
    }
    let mut buf_slice = &buf[..];
    let mut buf_u64 = [0; 8];
    io::Read::read_exact(&mut buf_slice, &mut buf_u64).unwrap();
    let sector_idx = u64::from_le_bytes(buf_u64);
    let mut buf_u64 = [0; 8];
    io::Read::read_exact(&mut buf_slice, &mut buf_u64).unwrap();
    let logical_timestamp = u64::from_le_bytes(buf_u64);
    let mut buf_u8 = [0; 1];
    io::Read::read_exact(&mut buf_slice, &mut buf_u8).unwrap();
    let write_rank = buf_u8[0];
    Ok((sector_idx, logical_timestamp, write_rank))
}

#[test]
fn test_coding() {
    let test = (10, 2, 200);
    assert_eq!(decode_filename(encode_filename(test)).unwrap(), test);
    let test2 = (10000, 232, 200);
    assert_eq!(decode_filename(encode_filename(test2)).unwrap(), test2);
}

async fn unpack_entry_result(
    path: PathBuf,
    entry_result: Result<DirEntry, io::Error>,
) -> Option<(u64, u64, u8)> {
    if let Ok(entry) = entry_result {
        let filename = entry.file_name().into_string();
        if filename.is_err() {
            return None;
        }
        let filename = filename.unwrap();
        // Delete temporary files:
        if filename.starts_with("tmpfile") {
            let mut to_delete_filepath = path.clone();
            to_delete_filepath.push(filename);
            let _ = fs::remove_file(to_delete_filepath);
            return None;
        }
        let tup_err = decode_filename(filename);
        if let Ok(tup) = tup_err {
            return Some(tup);
        } else {
            return None;
        }
    } else {
        return None;
    }
}

impl FileSystemSectorsManager {
    async fn new(path: PathBuf) -> Self {
        let mut meta = vec![HashMap::new(); NUMBER_OF_WORKERS];
        // This is not asynchronous but it is okay because
        // sectors manager is created before the algorithm
        // is runned.
        for entry_result in path.read_dir().expect("read_dir call failed") {
            if let Some((sector_idx, logical_timestamp, write_rank)) =
                unpack_entry_result(path.clone(), entry_result).await
            {
                let tup_opt = meta[(sector_idx as usize) % NUMBER_OF_WORKERS]
                    .get(&sector_idx)
                    .clone();
                if let Some(tup) = tup_opt {
                    if (logical_timestamp, write_rank) > *tup {
                        let old_filename = encode_filename((sector_idx, tup.0, tup.1));
                        meta[(sector_idx as usize) % NUMBER_OF_WORKERS]
                            .insert(sector_idx, (logical_timestamp, write_rank));
                        let mut old_filepath = path.clone();
                        old_filepath.push(old_filename);
                        let _ = fs::remove_file(old_filepath).await;
                    } else {
                        let to_delete_filename =
                            encode_filename((sector_idx, logical_timestamp, write_rank));
                        let mut to_delete_filepath = path.clone();
                        to_delete_filepath.push(to_delete_filename);
                        let _ = fs::remove_file(to_delete_filepath).await;
                    }
                    File::open(&path).await.unwrap().sync_data().await.unwrap();
                } else {
                    meta[(sector_idx as usize) % NUMBER_OF_WORKERS]
                        .insert(sector_idx, (logical_timestamp, write_rank));
                }
            }
        }
        FileSystemSectorsManager {
            path,
            idx_to_meta: meta.into_iter().map(|x| RwLock::new(x)).collect(),
        }
    }
}

#[async_trait::async_trait]
impl SectorsManager for FileSystemSectorsManager {
    async fn read_data(&self, idx: SectorIdx) -> SectorVec {
        let map = self.idx_to_meta[(idx as usize) % NUMBER_OF_WORKERS]
            .read()
            .await;
        if let Some((logical_timestamp, write_rank)) = map.get(&idx) {
            let filename = encode_filename((idx, *logical_timestamp, *write_rank));
            let mut filepath = self.path.clone();
            filepath.push(filename);
            let mut file = File::open(&filepath).await.unwrap();
            let mut content = vec![];
            file.read_to_end(&mut content).await.unwrap();
            if content.len() != 4096 {
                panic!("SectorsManager invariant doesn't hold");
            }
            SectorVec(content)
        } else {
            SectorVec(vec![0; SECTOR_LEN])
        }
    }

    async fn read_metadata(&self, idx: SectorIdx) -> (u64, u8) {
        let map = self.idx_to_meta[(idx as usize) % NUMBER_OF_WORKERS]
            .read()
            .await;
        if let Some(tup) = map.get(&idx) {
            tup.clone()
        } else {
            (0, 0)
        }
    }

    async fn write(&self, idx: SectorIdx, sector: &(SectorVec, u64, u8)) {
        let (SectorVec(content), logical_timestamp, write_rank) = sector;

        let mut map = self.idx_to_meta[(idx as usize) % NUMBER_OF_WORKERS]
            .write()
            .await;
        let mut old_filename = None;
        if let Some((logical_timestamp, write_rank)) = map.get(&idx) {
            old_filename = Some(encode_filename((idx, *logical_timestamp, *write_rank)));
        }

        let new_filename = encode_filename((idx, *logical_timestamp, *write_rank));
        let mut new_filepath = self.path.clone();
        new_filepath.push(new_filename.clone());

        let mut tmppath = self.path.clone();
        tmppath.push(format!("tmpfile{}", new_filename));

        let tmpfile_result = File::create(&tmppath).await;

        let mut tmpfile = tmpfile_result.unwrap();

        tmpfile.write_all(content).await.unwrap();

        tmpfile.sync_data().await.unwrap();

        drop(tmpfile);

        File::open(&self.path)
            .await
            .unwrap()
            .sync_data()
            .await
            .unwrap();

        fs::rename(&tmppath, &new_filepath).await.unwrap();

        let dest_file = File::open(&new_filepath).await.unwrap();
        dest_file.sync_data().await.unwrap();
        drop(dest_file);

        if let Some(old_filename) = old_filename {
            let mut old_filepath = self.path.clone();
            old_filepath.push(old_filename);
            if old_filepath != new_filepath {
                let _ = fs::remove_file(old_filepath).await;
            }
        }

        File::open(&self.path)
            .await
            .unwrap()
            .sync_data()
            .await
            .unwrap();

        map.insert(idx, (*logical_timestamp, *write_rank));
    }
}
