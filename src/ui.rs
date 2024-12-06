use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSRect, NSPoint, NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::signaling::SignalingMessage;
use objc::runtime::{Object, Sel};
use parking_lot::Mutex;
use parking_lot::Mutex as OnceMutex;
use std::sync::Once;
use once_cell::sync::Lazy;

pub struct Application {
    window: id,
    peer_list: id,
    tx: mpsc::UnboundedSender<SignalingMessage>,
    peers: Arc<Mutex<Vec<String>>>,
}

unsafe impl Send for Application {}
unsafe impl Sync for Application {}

static INSTANCE: Lazy<Mutex<Option<Arc<Application>>>> = Lazy::new(|| Mutex::new(None));
static INIT: Once = Once::new();

impl Application {
    pub fn new(tx: mpsc::UnboundedSender<SignalingMessage>) -> Arc<Self> {
        let instance = Arc::new(Self {
            window: nil,
            peer_list: nil,
            tx,
            peers: Arc::new(Mutex::new(Vec::new())),
        });
        
        *INSTANCE.lock() = Some(instance.clone());
        
        instance
    }
    
    pub fn get_instance() -> Option<Arc<Self>> {
        INSTANCE.lock().clone()
    }
    
    pub fn run(self: &Arc<Self>) {
        unsafe {
            let app: id = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![self.window, makeKeyAndOrderFront: nil];
            let _: () = msg_send![app, run];
        }
    }

    fn create_window() -> id {
        unsafe {
            let window: id = msg_send![class!(NSWindow), alloc];
            let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(400., 300.));
            let window: id = msg_send![
                window,
                initWithContentRect:frame
                styleMask:0xf
                backing:2
                defer:NO
            ];
            
            let title = NSString::alloc(nil).init_str("WebRTC Voice Chat");
            let _: () = msg_send![window, setTitle:title];
            let _: () = msg_send![window, center];
            
            window
        }
    }

    fn create_peer_list(window: id) -> id {
        unsafe {
            let frame = NSRect::new(NSPoint::new(20., 20.), NSSize::new(360., 260.));
            let scroll_view: id = msg_send![class!(NSScrollView), alloc];
            let scroll_view: id = msg_send![scroll_view, initWithFrame:frame];
            
            let table_view: id = msg_send![class!(NSTableView), alloc];
            let table_view: id = msg_send![table_view, initWithFrame:frame];
            
            // Set up double click handler
            let _: () = msg_send![table_view, setTarget:table_view];
            let _: () = msg_send![table_view, setDoubleAction:sel!(onDoubleClick:)];
            
            let column: id = msg_send![class!(NSTableColumn), alloc];
            let column: id = msg_send![column, initWithIdentifier: NSString::alloc(nil).init_str("peers")];
            let _: () = msg_send![column, setWidth: 340.];
            let _: () = msg_send![table_view, addTableColumn: column];
            
            let _: () = msg_send![scroll_view, setDocumentView: table_view];
            let _: () = msg_send![scroll_view, setHasVerticalScroller: YES];
            
            let content_view: id = msg_send![window, contentView];
            let _: () = msg_send![content_view, addSubview: scroll_view];
            
            // Register for double click notifications
            extern fn double_click_handler(this: &Object, _cmd: Sel, _sender: id) {
                unsafe {
                    let selected_row: i64 = msg_send![this, selectedRow];
                    if selected_row >= 0 {
                        if let Some(app) = Application::get_instance() {
                            app.handle_peer_selected(selected_row as usize);
                        }
                    }
                }
            }
            
            table_view
        }
    }

    pub fn update_peer_list(&self, peers: Vec<String>) {
        let mut peer_list = self.peers.lock();
        *peer_list = peers;
        unsafe {
            let _: () = msg_send![self.peer_list, reloadData];
        }
    }

    fn handle_peer_selected(&self, index: usize) {
        if let Some(peer_id) = self.get_peer_at_index(index) {
            // Send message to initiate call
            let _ = self.tx.send(SignalingMessage::InitiateCall {
                peer_id: peer_id.to_string(),
                room_id: "test-room".to_string(),
            });
        }
    }

    fn get_peer_at_index(&self, index: usize) -> Option<String> {
        self.peers.lock().get(index).cloned()
    }
}