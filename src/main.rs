use anyhow::Result;
use webrtc::api::APIBuilder;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use futures_util::{StreamExt, SinkExt};
use crate::signaling::SignalingMessage;
use tokio::sync::mpsc;
use uuid::Uuid;
use futures_util::stream::SplitSink;
use tokio_tungstenite::tungstenite::Message;
use parking_lot::Mutex;

mod signaling;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    
    // Initialize the WebRTC API
    let api = APIBuilder::new().build();
    let config = RTCConfiguration::default();
    let peer_connection = Arc::new(api.new_peer_connection(config).await?);
    
    // Generate a unique peer ID
    let peer_id = Uuid::new_v4().to_string();
    
    // Set up connection handlers in a separate task
    let peer_connection_clone = peer_connection.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = setup_signaling(peer_connection_clone, peer_id, tx_clone).await {
            eprintln!("Error in signaling: {}", e);
        }
    });

    // Initialize and run the UI on the main thread
    let app = ui::Application::new(tx);
    
    // Handle incoming messages from the UI
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            // Handle UI messages
            match msg {
                SignalingMessage::RequestPeerList => {
                    // Send request to server
                }
                SignalingMessage::InitiateCall { peer_id, room_id } => {
                    if let Err(e) = handle_call_initiation(
                        peer_connection.clone(),
                        peer_id,
                        room_id,
                        &mut write
                    ).await {
                        eprintln!("Error initiating call: {}", e);
                    }
                },
                _ => {}
            }
        }
    });

    app.run(); // This will block until the window is closed

    Ok(())
}

async fn setup_signaling(
    peer_connection: Arc<RTCPeerConnection>,
    peer_id: String,
    tx: mpsc::UnboundedSender<SignalingMessage>
) -> Result<()> {
    let ws_stream = connect_to_signaling_server("ws://127.0.0.1:8080").await?;
    let (write, mut read) = ws_stream.split();
    let write = Arc::new(Mutex::new(write));
    let write_clone = write.clone();

    // Join a room
    let join_msg = SignalingMessage::Join {
        room_id: "test-room".to_string(),
        peer_id: peer_id.clone(),
    };
    write.lock().await.send(tokio_tungstenite::tungstenite::Message::Text(
        serde_json::to_string(&join_msg)?,
    )).await?;

    while let Some(msg) = read.next().await {
        if let Ok(msg) = msg {
            if let Ok(text) = msg.to_text() {
                if let Ok(signal_msg) = serde_json::from_str::<SignalingMessage>(text) {
                    match signal_msg {
                        SignalingMessage::PeerList { peers } => {
                            // Forward peer list to UI
                            tx.send(SignalingMessage::PeerList { peers })?;
                        },
                        SignalingMessage::Offer { sdp, .. } => {
                            let desc = webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(sdp)?;
                            peer_connection.set_remote_description(desc).await?;
                            
                            let answer = peer_connection.create_answer(None).await?;
                            peer_connection.set_local_description(answer.clone()).await?;
                            
                            let answer_msg = SignalingMessage::Answer {
                                room_id: "test-room".to_string(),
                                sdp: answer.sdp,
                                from_peer: peer_id.clone(),
                                to_peer: "".to_string(),
                            };
                            write.lock().await.send(tokio_tungstenite::tungstenite::Message::Text(
                                serde_json::to_string(&answer_msg)?,
                            )).await?;
                        },
                        SignalingMessage::Answer { sdp, .. } => {
                            let desc = webrtc::peer_connection::sdp::session_description::RTCSessionDescription::answer(sdp)?;
                            peer_connection.set_remote_description(desc).await?;
                        },
                        SignalingMessage::IceCandidate { candidate, .. } => {
                            let candidate_init: webrtc::ice_transport::ice_candidate::RTCIceCandidateInit = 
                                serde_json::from_str(&candidate)?;
                            peer_connection.add_ice_candidate(candidate_init).await?;
                        },
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

async fn connect_to_signaling_server(url: &str) -> Result<WebSocketStream<TcpStream>> {
    let addr = url.trim_start_matches("ws://");
    let stream = TcpStream::connect(addr).await?;
    let ws_stream = tokio_tungstenite::client_async(url, stream).await?.0;
    Ok(ws_stream)
}

async fn handle_call_initiation(
    peer_connection: Arc<RTCPeerConnection>,
    peer_id: String,
    room_id: String,
    write: &mut SplitSink<WebSocketStream<TcpStream>, Message>
) -> Result<()> {
    // Create offer
    let offer = peer_connection.create_offer(None).await?;
    peer_connection.set_local_description(offer.clone()).await?;
    
    // Send offer through signaling server
    let offer_msg = SignalingMessage::Offer {
        room_id,
        sdp: offer.sdp,
        from_peer: peer_id,
        to_peer: peer_id,
    };
    
    write.send(Message::Text(serde_json::to_string(&offer_msg)?)).await?;
    Ok(())
}
