use std::{io::Read, sync::RwLock, rc::Rc};

use flate2::read::DeflateDecoder;
use wasm_bindgen::{prelude::*, Clamped};
use web_sys::{MessageEvent, WebSocket, ErrorEvent, HtmlCanvasElement, CanvasRenderingContext2d, ImageData, KeyboardEvent, MouseEvent, Event, WheelEvent};
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
    let mut rgb_img = Vec::<u8>::new();
    let mut real_img = Vec::<u8>::new();
    let mut temp = Vec::<u8>::new();
    let (mut w, mut h, mut dlen) = (0u32, 0u32, 0usize);
    let (fw, fh) = (Rc::new(RwLock::new(0u32)), Rc::new(RwLock::new(0u32)));
    let canvas1 = canvas.clone();
    let (tfw, tfh) = (fw.clone(), fh.clone());
    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&abuf);
            let data = array.to_vec();
            let len = array.byte_length() as usize;
            if len == 4 {
                // 初始化w, h
                w = ((data[0] as u32) << 8) | (data[1] as u32);
                h = ((data[2] as u32) << 8) | (data[3] as u32);
                if let Ok(mut tfw) = tfw.write() {
                    *tfw = w;
                }
                if let Ok(mut tfh) = tfh.write() {
                    *tfh = h;
                }
                canvas1.set_width(w);
                canvas1.set_height(h);
                dlen = (w * h) as usize * 3;
                let rlen = (w * h * 4) as usize;
                real_img = Vec::<u8>::with_capacity(rlen);
                unsafe {
                    real_img.set_len(rlen);
                }
                temp = Vec::with_capacity(dlen);
                unsafe {
                    temp.set_len(dlen);
                }
                console_log!("w = {} h = {} dlen = {}", w, h, dlen);
            } else if rgb_img.len() == 0 {
                rgb_img = Vec::with_capacity(dlen);
                unsafe {
                    rgb_img.set_len(dlen);
                }
                // 接收原图像
                let mut dec = DeflateDecoder::new(&data[..]);
                dec.read_exact(&mut temp).unwrap();
                rgb_img.copy_from_slice(&temp);
                real_img.chunks_exact_mut(4).zip(rgb_img.chunks_exact(3)).for_each(|(c, b)|{
                    (c[0], c[1], c[2], c[3]) = (b[0], b[1], b[2], 255u8);
                });
                let im = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&real_img), w, h).unwrap();
                ctx.put_image_data(&im, 0.0, 0.0).unwrap();
            } else {
                // 接收差异图像
                let mut dec = DeflateDecoder::new(&data[..]);
                dec.read_exact(&mut temp).unwrap();
                rgb_img.iter_mut().zip(temp.iter()).for_each(|(d1, d2)| {
                    *d1 ^= *d2;
                });
                real_img.chunks_exact_mut(4).zip(rgb_img.chunks_exact(3)).for_each(|(c, b)|{
                    (c[0], c[1], c[2], c[3]) = (b[0], b[1], b[2], 255u8);
                });
                let im = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&real_img), w, h).unwrap();
                ctx.put_image_data(&im, 0.0, 0.0).unwrap();
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
    }) as Box<dyn FnMut(MouseEvent)> );
    canvas.set_onmouseover(Some(mouseover.as_ref().unchecked_ref()));
    mouseover.forget();

    // 鼠标离开失去焦点
    let cavas3 = canvas.clone();
    let mouseout = Closure::wrap(Box::new(move |_: MouseEvent| {
        cavas3.blur().unwrap();
    }) as Box<dyn FnMut(MouseEvent)> );
    canvas.set_onmouseout(Some(mouseout.as_ref().unchecked_ref()));
    mouseout.forget();

    // 禁止右键弹出菜单
    let contextmenu = Closure::wrap(Box::new(move |e: Event| {
        e.prevent_default();
        e.stop_propagation();
    }) as Box<dyn FnMut(Event)> );
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
    }) as Box<dyn FnMut(WheelEvent)> );
    canvas.set_onwheel(Some(wheel.as_ref().unchecked_ref()));
    wheel.forget();

    // 鼠标按下
    let tws = ws.clone();
    let mousedown = Closure::wrap(Box::new(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        let btn = e.button();
        tws.send_with_u8_array(&[dscom::MOUSE_KEY_DOWN, btn as u8]).unwrap();
        // console_log!("mousedown {}", btn);
    }) as Box<dyn FnMut(MouseEvent)> );
    canvas.set_onmousedown(Some(mousedown.as_ref().unchecked_ref()));
    mousedown.forget();

    // 鼠标弹起
    let tws = ws.clone();
    let mouseup = Closure::wrap(Box::new(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        let btn = e.button();
        tws.send_with_u8_array(&[dscom::MOUSE_KEY_UP, btn as u8]).unwrap();
        // console_log!("mouseup {}", btn);
    }) as Box<dyn FnMut(MouseEvent)> );
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
        tws.send_with_u8_array(&[dscom::MOVE, (x>>8) as u8, x as u8, (y>>8) as u8, y as u8]).unwrap();
        // console_log!("mousemove {} {}", x, y);
    }) as Box<dyn FnMut(MouseEvent)> );
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
        tws.send_with_u8_array(&[dscom::KEY_UP, code as u8]).unwrap();
        // console_log!("keyup {}", code);
    }) as Box<dyn FnMut(KeyboardEvent)>);
    canvas.set_onkeyup(Some(keyup.as_ref().unchecked_ref()));
    keyup.forget();


    Ok(ws)
}