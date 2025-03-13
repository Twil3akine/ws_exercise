use std::{
	collections::{
		HashMap,
		HashSet,
	},
	sync::Arc,
	net::SocketAddr
};
use axum::{
	extract::{
		ws::{ Message, WebSocket, WebSocketUpgrade },
		connect_info::ConnectInconnect_info::ConnectInfofo,
		State,
	},
	response::IntoResponse,
	routing::get,
	Router,
};
use futures::{
	StreamExt,
	SinkExt
};
use tokio::{
	net::TcpListener,
	sync::{
		mpsc::{
			Sender,
			channel,
		},
		Mutex,
		oneshot,
	}
};
use serde::{ Deserialize, Serialize };

type Room = String;
type Peer = Sender<Message>;
type PeerKey = (Room, SocketAddr);

#[derive(Clone)]
struct AppState {
	rooms: Arc<Mutex<HashMap<PeerKey, Peer>>>;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum MessagePayload {
	Join { rooms: String },
	Leave { rooms: String },
	Broadcast { rooms: String, message: String },
}

#[tokio::main]
async fn main() {
	let state = AppState {
		rooms: Arc::new(Mutex::new(HashMap::new())),
	};

	let app = Router::new()
			.route("/ws", get(ws_handler))
			.with_state(state);

	let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
	let listener = TcpListener::bind(addr).await.unwrap();

	println!("Server running on ws://{}", addr);

	axum::serve(
		listener,
		app.into_make_service_with_connect_info::<SocketAddr>(),
	)
	.await
	.unwrap();
}

async fn ws_handler(
	ws: WebSocketUpgrade,
	ConnectInfo(addr): ConnectInfo<SocketAddr>,
	State(state): State<AppState>
) -> impl IntoResponse {
	ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr, state: AppState) {
	println!("{who} connected.");

	let (mut sender, mut receiver) = socket.split();
	let (tx, mut rx) = channel::<Message>(128);
	let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();
	let mut joined_rooms: HashSet<Room> = HashSet::new();

	let recv_task = tokio::spawn(async move {
		while let Some(msg) = receiver.next().await {
			let msg = match msg {
				Ok(msg) => msg,
				Err(e) => {
					eprintln!("Error receiving message: {:?}", e);
					break;
				}
			};

			let msg = match msg {
				Message::Text(text) => text,
				_ => continue,
			};

			let payload: MessagePayload = match serde_json::from_str(&msg) {
				Ok(payload) => payload,
				Err(e) => {
					eprintln!("Error parsing message: {:?}", e);
					continue;
				}
			};
	
			match payload {
				MessagePayload::Join { room } => {
					add_peer(&state, (room.clone(), who), tx_clone()).await;
					joined_rooms.insert(room.clone());
					println!("{who} joined room {room}");
				},
				MessagePayload::Leave { room } => {
					remove_peer(&state, (room.clone(), who)).await;
					joined_rooms.remove(&room);
					println!("{who} left room {room}");
				},
				_ => (),
			}
		}

		for room in joined_rooms {
			remove_peer(&state, (room, who)).await;
		}

		let _ = cancel_tx.send(());
	});

	let send_task = tokio::spawn(async move {
		loop {
			tokio::select! {
				Some(msg) = rx.recv() => {
					if let Err(e) = sender.send(msg).await {
						eprintln!("Error sending message: {:?}", e);
						break;
					}
					_ = &mut cancel_rx => {
						break;
					}
				}
			}
		}
	});

	tokio::select! {
		res = send_task => {
			if let Err(e) = res {
				eprintln!("Send task panicked: {:?}", e);
			}
		}

		res = recv_task => {
			if let Err(e) = res {
				eprintln!("Receive task panicked: {:?}", e);
			}
		}
	}
}

async fn add_peer(state: &AppState, peer_key: PeerKey, peer: Peer) {
	let mut rooms: state.rooms.lock().await;
	rooms.insert(peer_key, peer);
}

async fn remove_peer(state: &AppState, peer_key: PeerKey) {
	let mut rooms = state.rooms.lock().await;
	rooms.remove(&peer_key);
}
