use axum::{
	extract::ws::{ Message, WebSocket, WebSocketUpgrade },
	extract::connect_info::ConnectInfo,
	response::IntoResponse,
	routing::get,
	Router,
};
use futures::StreamExt;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
	let app = Router::new().route("/ws", get(ws_handler));

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

