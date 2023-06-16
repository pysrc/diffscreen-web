use std::{rc::Rc, sync::RwLock, io::Write};

use flate2::write::DeflateDecoder;
use wasm_bindgen::{prelude::*, Clamped};
use web_sys::{
    CanvasRenderingContext2d, ErrorEvent, Event, HtmlCanvasElement, ImageData, KeyboardEvent,
    MessageEvent, MouseEvent, WebSocket, WheelEvent,
};
mod bitmap;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn start_websocket(canvas_id: &str, host: &str) -> Result<WebSocket, JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id(canvas_id).unwrap();
    let canvas: HtmlCanvasElement = canvas
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    canvas.set_tab_index(1);
    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .unwrap();
    let ws: WebSocket = WebSocket::new_with_str(host, "diffscreen")?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    let mut real_img = Vec::<u8>::new();
    let (mut sw, mut sh, mut srw) = (0u32, 0u32, 0u32);
    let (fw, fh) = (Rc::new(RwLock::new(0u32)), Rc::new(RwLock::new(0u32)));
    let canvas1 = canvas.clone();
    let (tfw, tfh) = (fw.clone(), fh.clone());
    let mut bit_mask = 0u8;
    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&abuf);
            let data = array.to_vec();
            let len = array.byte_length() as usize;
            if len == 9 {
                // 初始化w, h
                let w = ((data[0] as u32) << 8) | (data[1] as u32);
                let h = ((data[2] as u32) << 8) | (data[3] as u32);
                sw = ((data[4] as u32) << 8) | (data[5] as u32);
                sh = ((data[6] as u32) << 8) | (data[7] as u32);
                bit_mask = data[8];
                srw = (w / sw) + if w % sw == 0 {0u32} else {1u32};
                if let Ok(mut tfw) = tfw.write() {
                    *tfw = w;
                }
                if let Ok(mut tfh) = tfh.write() {
                    *tfh = h;
                }
                canvas1.set_width(w);
                canvas1.set_height(h);
                console_log!("w = {} h = {}", w, h);
                real_img = vec![0u8; (sw * sh * 4) as usize];
            } else {
                // 接收原图像
                let mut row_recv = Vec::with_capacity(1024 * 8);
                let mut e = DeflateDecoder::new(row_recv);
                e.write_all(&data).unwrap();
                row_recv = e.finish().unwrap();
                let mut row_recv = &row_recv[..];
                let end = (sw * sh * 3) as usize + 2;
                while row_recv.len() > 0 {
                    let temp = &row_recv[..end];
                    row_recv = &row_recv[end..];
                    let k = ((temp[0] as usize) << 8) | (temp[1] as usize);
                    real_img
                        .chunks_exact_mut(4)
                        .zip((temp[2..]).chunks_exact(3))
                        .for_each(|(c, b)| {
                            (c[0], c[1], c[2], c[3]) = (b[0] << bit_mask, b[1] << bit_mask, b[2] << bit_mask, 255u8);
                        });
                    let im = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&real_img), sw, sh)
                        .unwrap();
                    let ih = sh * (k as u32 / srw);
                    let iw = sw * (k as u32 % srw);
                    ctx.put_image_data(&im, iw as f64, ih as f64).unwrap();
                }
            }
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
    let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
        console_log!("error event: {:?}", e);
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

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

    Ok(ws)
}
