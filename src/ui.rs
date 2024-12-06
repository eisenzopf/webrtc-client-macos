use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSRect, NSPoint, NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::signaling::SignalingMessage;
use objc::runtime::{Object, Sel};
use parking_lot::Mutex;
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
        let window = Self::create_window();
        let peer_list = Self::create_peer_list(window);
        
        let instance = Arc::new(Self {
            window,
            peer_list,
            tx: tx.clone(),
            peers: Arc::new(Mutex::new(Vec::new())),
        });
        
        *INSTANCE.lock() = Some(instance.clone());
        
        let tx_clone = tx;
        tokio::spawn(async move {
            let _ = tx_clone.send(SignalingMessage::RequestPeerList);
        });
        
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
            let style_mask = 
                1 << 0 |  // NSWindowStyleMaskTitled
                1 << 1 |  // NSWindowStyleMaskClosable
                1 << 2 |  // NSWindowStyleMaskMiniaturizable
                1 << 3;   // NSWindowStyleMaskResizable
            
            let window: id = msg_send![window,
                initWithContentRect:frame
                styleMask:style_mask
                backing:2  // NSBackingStoreBuffered
                defer:NO
            ];
            
            let title = NSString::alloc(nil).init_str("WebRTC Voice Chat");
            let _: () = msg_send![window, setTitle:title];
            let _: () = msg_send![window, center];
            
            // Make sure the window is visible
            let _: () = msg_send![window, setIsVisible:YES];
            
            window
        }
    }

    fn create_peer_list(window: id) -> id {
        unsafe {
            let content_view: id = msg_send![window, contentView];
            let content_frame: NSRect = msg_send![content_view, frame];
            
            // Create frame for scroll view with some padding
            let frame = NSRect::new(
                NSPoint::new(20.0, 20.0),
                NSSize::new(
                    content_frame.size.width - 40.0,
                    content_frame.size.height - 40.0
                )
            );
            
            let scroll_view: id = msg_send![class!(NSScrollView), alloc];
            let scroll_view: id = msg_send![scroll_view, initWithFrame:frame];
            
            // Set scroll view properties
            let _: () = msg_send![scroll_view, setBorderType:0];
            let _: () = msg_send![scroll_view, setHasVerticalScroller:YES];
            let _: () = msg_send![scroll_view, setHasHorizontalScroller:NO];
            let _: () = msg_send![scroll_view, setAutohidesScrollers:YES];
            
            let table_view: id = msg_send![class!(NSTableView), alloc];
            let table_view: id = msg_send![table_view, initWithFrame:frame];
            
            // Configure table view
            let _: () = msg_send![table_view, setAllowsMultipleSelection:NO];
            let _: () = msg_send![table_view, setColumnAutoresizingStyle:1];
            
            let class_name = "NSTableViewDelegate";
            let superclass = class!(NSObject);
            let mut decl = objc::declare::ClassDecl::new(class_name, superclass).unwrap();

            decl.add_method(
                sel!(numberOfRowsInTableView:),
                number_of_rows as extern "C" fn(&Object, Sel, id) -> i64,
            );
            decl.add_method(
                sel!(tableView:objectValueForTableColumn:row:),
                object_value_for_table_column as extern "C" fn(&Object, Sel, id, id, i64) -> id,
            );
            decl.add_method(
                sel!(tableViewSelectionDidChange:),
                on_double_click as extern "C" fn(&Object, Sel, id) -> (),
            );

            let delegate_class = decl.register();
            let delegate: id = msg_send![delegate_class, new];
            
            // Set the data source and delegate
            let _: () = msg_send![table_view, setDataSource:delegate];
            let _: () = msg_send![table_view, setDelegate:delegate];
            
            let column: id = msg_send![class!(NSTableColumn), alloc];
            let column: id = msg_send![column, initWithIdentifier:NSString::alloc(nil).init_str("peers")];
            let _: () = msg_send![column, setWidth:frame.size.width - 20.0];
            let _: () = msg_send![table_view, addTableColumn:column];
            
            // Set up the document view
            let _: () = msg_send![scroll_view, setDocumentView:table_view];
            
            // Add scroll view to window's content view
            let _: () = msg_send![content_view, addSubview:scroll_view];
            
            // Set up autoresizing masks
            let _: () = msg_send![scroll_view, setAutoresizingMask:18];
            let _: () = msg_send![table_view, setAutoresizingMask:18];
            
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

extern "C" fn number_of_rows(_this: &Object, _sel: Sel, _table_view: id) -> i64 {
    if let Some(app) = Application::get_instance() {
        app.peers.lock().len() as i64
    } else {
        0
    }
}

extern "C" fn object_value_for_table_column(
    _this: &Object,
    _sel: Sel,
    _table_view: id,
    _column: id,
    row: i64,
) -> id {
    unsafe {
        if let Some(app) = Application::get_instance() {
            let peers = app.peers.lock();
            if let Some(peer) = peers.get(row as usize) {
                return NSString::alloc(nil).init_str(peer);
            }
        }
        nil
    }
}

extern "C" fn on_double_click(_this: &Object, _sel: Sel, sender: id) -> () {
    unsafe {
        if let Some(app) = Application::get_instance() {
            let row: i64 = msg_send![sender, clickedRow];
            if row >= 0 {
                app.handle_peer_selected(row as usize);
            }
        }
    }
}