use crate::{SectorIdx, SectorVec, SectorsManager, StableStorage};
use log::*;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SectorMetadata {
    pub(crate) ts: u64,
    pub(crate) wr: u8,
}

pub(crate) struct SolutionAtomicRegisterData {
    meta_cache: HashMap<SectorIdx, SectorMetadata>,
    rid_cache: Option<u64>,
    rid_storage: Box<dyn StableStorage>,
    sectors_manager: Arc<dyn SectorsManager>,
}

impl SolutionAtomicRegisterData {
    pub(crate) async fn get_meta(&mut self, sector_idx: SectorIdx) -> SectorMetadata {
        if let Some(meta) = self.meta_cache.get(&sector_idx) {
            return meta.clone();
        } else {
            let ret = self.sectors_manager.read_metadata(sector_idx).await;
            let meta = SectorMetadata {
                ts: ret.0,
                wr: ret.1,
            };
            self.meta_cache.insert(sector_idx, meta.clone());
            return meta;
        }
    }

    pub(crate) async fn get_val(&mut self, sector_idx: SectorIdx) -> SectorVec {
        self.sectors_manager.read_data(sector_idx).await
    }

    pub(crate) async fn get_rid(&mut self) -> u64 {
        if self.rid_cache.is_none() {
            if let Some(bytes) = self.rid_storage.get("rid").await {
                if bytes.len() == 8 {
                    self.rid_cache = Some(read_be_u64(&mut &bytes[..]));
                } else {
                    error!("Corrupted rid file, reseting rid");
                    self.put_rid(0).await;
                }
            } else {
                self.put_rid(0).await;
            }
        }
        self.rid_cache.unwrap()
    }

    pub(crate) async fn put_rid(&mut self, new_value: u64) {
        self.rid_cache = Some(new_value);
        self.rid_storage
            .put("rid", &new_value.to_be_bytes())
            .await
            .unwrap();
    }

    pub(crate) async fn put_val_and_meta(
        &mut self,
        sector_idx: SectorIdx,
        val: SectorVec,
        meta: &SectorMetadata,
    ) {
        self.sectors_manager
            .write(sector_idx, &(val, meta.ts, meta.wr))
            .await;
        self.meta_cache.insert(sector_idx, meta.clone());
    }

    pub(crate) fn new(
        metadata: Box<dyn StableStorage>,
        sectors_manager: Arc<dyn SectorsManager>,
    ) -> SolutionAtomicRegisterData {
        SolutionAtomicRegisterData {
            meta_cache: HashMap::new(),
            rid_cache: None,
            rid_storage: metadata,
            sectors_manager,
        }
    }
}

fn read_be_u64(input: &mut &[u8]) -> u64 {
    let (int_bytes, rest) = input.split_at(std::mem::size_of::<u64>());
    *input = rest;
    u64::from_be_bytes(int_bytes.try_into().unwrap())
}
