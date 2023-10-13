use std::{rc::Rc, sync::RwLock, io::Write};

use flate2::write::DeflateDecoder;
use wasm_bindgen::{prelude::*, Clamped};
use web_sys::{
    CanvasRenderingContext2d, Event, HtmlCanvasElement, ImageData, KeyboardEvent,
    MessageEvent, MouseEvent, WebSocket, WheelEvent
};
mod bitmap;
mod convert;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// macro_rules! console_log {
//     ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
// }

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn start_websocket(canvas_id: &str, host: &str, width: usize, height: usize, n: usize, m: usize) -> Result<(), JsValue> {
    let subimage_width = width / n;
    let subimage_height = height / m;
    let subplane_size = subimage_width * subimage_height;

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id(canvas_id).unwrap();
    let canvas: HtmlCanvasElement = canvas
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    canvas.set_tab_index(1);
    canvas.set_width(width as u32);
    canvas.set_height(height as u32);

    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .unwrap();
    for k in 0..n * m {
        let screenurl = format!("ws://{}/diffscreen/{}", host, k);
        let ws: WebSocket = WebSocket::new_with_str(&screenurl, "diffscreen").unwrap();
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
        let ctx = ctx.clone();
        // 真实的所在列
        let rm = ((k % n) * subimage_width) as f64;
        // 真实的所在行
        let rn = ((k / n) * subimage_height) as f64;
        let ws1 = ws.clone();
        let onmsg = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);
                let cpr = array.to_vec();
                let mut row_recv = Vec::with_capacity(1024 * 8);
                let mut e = DeflateDecoder::new(row_recv);
                e.write_all(&cpr).unwrap();
                row_recv = e.finish().unwrap();
                // 解码成rgba
                let mut rgba = vec![0u8; subplane_size * 4];
                convert::i420_to_rgba(subimage_width, subimage_height, &row_recv[..subplane_size], &row_recv[subplane_size..subplane_size + subplane_size/4], &row_recv[subplane_size + subplane_size/4..], &mut rgba);
                let im = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&rgba), subimage_width as u32, subimage_height as u32)
                        .unwrap();
                // 画子图像
                ctx.put_image_data(&im, rm, rn).unwrap();
                ws1.send_with_str("S").unwrap();
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmsg.as_ref().unchecked_ref()));
        onmsg.forget();
        
    }
    let ctrlurl = format!("ws://{}/ctrl", host);
    let ws: WebSocket = WebSocket::new_with_str(&ctrlurl, "ctrl").unwrap();
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    let (fw, fh) = (Rc::new(RwLock::new(width as u32)), Rc::new(RwLock::new(height as u32)));

    // 鼠标悬停，获取焦点
    let cavas2 = canvas.clone();
    let mouseover = Closure::wrap(Box::new(move |_: MouseEvent| {
        cavas2.focus().unwrap();
    }) as Box<dyn FnMut(MouseEvent)>);
    canvas.set_onmouseover(Some(mouseover.as_ref().unchecked_ref()));
    mouseover.forget();

    // 鼠标离开失去焦点
    let cavas3 = canvas.clone();
    let mouseout = Closure::wrap(Box::new(move |_: MouseEvent| {
        cavas3.blur().unwrap();
    }) as Box<dyn FnMut(MouseEvent)>);
    canvas.set_onmouseout(Some(mouseout.as_ref().unchecked_ref()));
    mouseout.forget();

    // 禁止右键弹出菜单
    let contextmenu = Closure::wrap(Box::new(move |e: Event| {
        e.prevent_default();
        e.stop_propagation();
    }) as Box<dyn FnMut(Event)>);
    canvas.set_oncontextmenu(Some(contextmenu.as_ref().unchecked_ref()));
    contextmenu.forget();

    // 滚轮事件
    let tws = ws.clone();
    let wheel = Closure::wrap(Box::new(move |e: WheelEvent| {
        e.prevent_default();
        e.stop_propagation();
        if e.delta_y() < 0.0 {
            tws.send_with_u8_array(&[dscom::MOUSE_WHEEL_UP]).unwrap();
        } else {
            tws.send_with_u8_array(&[dscom::MOUSE_WHEEL_DOWN]).unwrap();
        }
        // console_log!("wheel {}", e.delta_y());
    }) as Box<dyn FnMut(WheelEvent)>);
    canvas.set_onwheel(Some(wheel.as_ref().unchecked_ref()));
    wheel.forget();

    // 鼠标按下
    let tws = ws.clone();
    let mousedown = Closure::wrap(Box::new(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        let btn = e.button();
        tws.send_with_u8_array(&[dscom::MOUSE_KEY_DOWN, btn as u8])
            .unwrap();
        // console_log!("mousedown {}", btn);
    }) as Box<dyn FnMut(MouseEvent)>);
    canvas.set_onmousedown(Some(mousedown.as_ref().unchecked_ref()));
    mousedown.forget();

    // 鼠标弹起
    let tws = ws.clone();
    let mouseup = Closure::wrap(Box::new(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        let btn = e.button();
        tws.send_with_u8_array(&[dscom::MOUSE_KEY_UP, btn as u8])
            .unwrap();
        // console_log!("mouseup {}", btn);
    }) as Box<dyn FnMut(MouseEvent)>);
    canvas.set_onmouseup(Some(mouseup.as_ref().unchecked_ref()));
    mouseup.forget();

    // 鼠标移动
    let vcan = canvas.clone();
    let tws = ws.clone();
    let mousemove = Closure::wrap(Box::new(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        let (mut x, mut y) = (0u32, 0u32);
        if let Ok(fw) = fw.read() {
            x = (*fw as f32 * e.offset_x() as f32 / vcan.client_width() as f32) as u32;
        }
        if let Ok(fh) = fh.read() {
            y = (*fh as f32 * e.offset_y() as f32 / vcan.client_height() as f32) as u32;
        }
        tws.send_with_u8_array(&[
            dscom::MOVE,
            (x >> 8) as u8,
            x as u8,
            (y >> 8) as u8,
            y as u8,
        ])
        .unwrap();
        // console_log!("mousemove {} {}", x, y);
    }) as Box<dyn FnMut(MouseEvent)>);
    canvas.set_onmousemove(Some(mousemove.as_ref().unchecked_ref()));
    mousemove.forget();

    // 键盘按下
    let bmap = bitmap::Bitmap::new();
    let bmap = Rc::new(RwLock::new(bmap));
    let tbmap = bmap.clone();
    let tws = ws.clone();
    let keydown = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        e.prevent_default();
        e.stop_propagation();
        let code = e.key_code() as u8;
        if let Ok(mut tbmap) = tbmap.write() {
            if tbmap.push(code) {
                tws.send_with_u8_array(&[dscom::KEY_DOWN, code]).unwrap();
            }
        }
        // console_log!("keydown {}", code);
    }) as Box<dyn FnMut(KeyboardEvent)>);
    canvas.set_onkeydown(Some(keydown.as_ref().unchecked_ref()));
    keydown.forget();

    // 键盘弹起
    let tws = ws.clone();
    let keyup = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        e.prevent_default();
        e.stop_propagation();
        let code = e.key_code() as u8;
        if let Ok(mut bmap) = bmap.write() {
            bmap.remove(code);
        }
        tws.send_with_u8_array(&[dscom::KEY_UP, code as u8])
            .unwrap();
        // console_log!("keyup {}", code);
    }) as Box<dyn FnMut(KeyboardEvent)>);
    canvas.set_onkeyup(Some(keyup.as_ref().unchecked_ref()));
    keyup.forget();

    Ok(())
}
