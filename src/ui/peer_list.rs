use cocoa::base::{id, nil};
use cocoa::foundation::NSString;
use objc::{class, msg_send, sel, sel_impl};
use std::sync::Arc;
use parking_lot::RwLock;

pub struct PeerListDataSource {
    peers: Arc<RwLock<Vec<String>>>,
}

impl PeerListDataSource {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn update_peers(&self, new_peers: Vec<String>) {
        let mut peers = self.peers.write();
        *peers = new_peers;
    }
}

unsafe impl objc::Message for PeerListDataSource {}

#[allow(non_snake_case)]
impl PeerListDataSource {
    extern "C" fn numberOfRowsInTableView_(_: &Self, _: objc::sel::Sel, _: id) -> i32 {
        let peers = self.peers.read();
        peers.len() as i32
    }

    extern "C" fn tableView_objectValueForTableColumn_row(
        &self,
        _: objc::sel::Sel,
        _: id,
        _: id,
        row: i32,
    ) -> id {
        let peers = self.peers.read();
        if let Some(peer) = peers.get(row as usize) {
            unsafe {
                NSString::alloc(nil).init_str(peer)
            }
        } else {
            nil
        }
    }
} 