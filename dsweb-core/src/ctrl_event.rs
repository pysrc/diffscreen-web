use enigo::{Enigo, KeyboardControllable, MouseControllable};
use futures_util::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::Message;

use crate::key_mouse;


pub async fn ctrl(mut rx: UnboundedReceiverStream<Message>) {
    let mut enigo = Enigo::new();
    while let Some(message) = rx.next().await {
        if message.is_close() {
            return;
        }
        if message.is_binary() {
            let cmd = message.into_bytes();
            match cmd[0] {
                dscom::KEY_UP => {
                    if let Some(key) = key_mouse::key_to_enigo(cmd[1]) {
                        enigo.key_up(key);
                    }
                }
                dscom::KEY_DOWN => {
                    if let Some(key) = key_mouse::key_to_enigo(cmd[1]) {
                        enigo.key_down(key);
                    }
                }
                dscom::MOUSE_KEY_UP => {
                    if let Some(key) = key_mouse::mouse_to_engin(cmd[1]) {
                        enigo.mouse_up(key);
                    }
                }
                dscom::MOUSE_KEY_DOWN => {
                    if let Some(key) = key_mouse::mouse_to_engin(cmd[1]) {
                        enigo.mouse_down(key);
                    }
                }
                dscom::MOUSE_WHEEL_UP => {
                    enigo.mouse_scroll_y(-2);
                }
                dscom::MOUSE_WHEEL_DOWN => {
                    enigo.mouse_scroll_y(2);
                }
                dscom::MOVE => {
                    let x = ((cmd[1] as i32) << 8) | (cmd[2] as i32);
                    let y = ((cmd[3] as i32) << 8) | (cmd[4] as i32);
                    enigo.mouse_move_to(x, y);
                }
                _ => {
                    return;
                }
            }
        }
    }
}