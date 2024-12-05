use crate::*;
use std::path::PathBuf;

pub(crate) struct Context {
    config: Configuration,
}

impl Context {
    pub(crate) fn new(config: Configuration) -> Self {
        Self { config }
    }

    pub(crate) fn self_addr(&self) -> &(String, u16) {
        &self.config.public.tcp_locations[(self.config.public.self_rank - 1) as usize]
    }

    pub(crate) fn self_rank(&self) -> &u8 {
        &self.config.public.self_rank
    }

    pub(crate) fn tcp_locations(&self) -> &Vec<(String, u16)> {
        &self.config.public.tcp_locations
    }

    pub(crate) fn hmac_system_key(&self) -> &[u8; 64] {
        &self.config.hmac_system_key
    }

    pub(crate) fn hmac_client_key(&self) -> &[u8; 32] {
        &self.config.hmac_client_key
    }

    pub(crate) fn storage_dir(&self) -> &PathBuf {
        &self.config.public.storage_dir
    }

    pub(crate) fn processes_count(&self) -> u8 {
        self.config.public.tcp_locations.len() as u8
    }

    pub(crate) fn n_sectors(&self) -> u64 {
        self.config.public.n_sectors
    }
}
