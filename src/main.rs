use std::{
	collections::HashMap,
	sync::Arc
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
use futures::StreamExt;
use std::net::SocketAddr;
use tokio::{
	net::TcpListener,
	sync::{
		mpsc::Sender,
		Mutex
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

async fn handle_socket(mut socket: WebSocket, who: SocketAddr) {
	println!("{who} connected.");

	while let Some(Ok(msg)) = socket.next().await {
		if let Message::Text(text) = msg {
			if socket.send(Message::Text(text)).await.is_err() {
				break;
			}
		}
	}
}

