use std::io::Read;

use flate2::read::DeflateDecoder;
use wasm_bindgen::{prelude::*, Clamped};
use web_sys::{MessageEvent, WebSocket, ErrorEvent, HtmlCanvasElement, CanvasRenderingContext2d, ImageData, KeyboardEvent, MouseEvent, Event, WheelEvent};

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
    let ws = WebSocket::new_with_str(host, "diffscreen")?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);
    let mut rgb_img = Vec::<u8>::new();
    let mut real_img = Vec::<u8>::new();
    let mut temp = Vec::<u8>::new();
    let (mut w, mut h, mut dlen) = (0u32, 0u32, 0usize);
    let canvas1 = canvas.clone();
    let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&abuf);
            let data = array.to_vec();
            let len = array.byte_length() as usize;
            if len == 4 {
                // 初始化w, h
                w = ((data[0] as u32) << 8) | (data[1] as u32);
                h = ((data[2] as u32) << 8) | (data[3] as u32);
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
    let wheel = Closure::wrap(Box::new(move |e: WheelEvent| {
        e.prevent_default();
        e.stop_propagation();
        console_log!("wheel {}", e.delta_y());
    }) as Box<dyn FnMut(WheelEvent)> );
    canvas.set_onwheel(Some(wheel.as_ref().unchecked_ref()));
    wheel.forget();

    // 鼠标按下
    let mousedown = Closure::wrap(Box::new(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        let btn = e.button();
        console_log!("mousedown {}", btn);
    }) as Box<dyn FnMut(MouseEvent)> );
    canvas.set_onmousedown(Some(mousedown.as_ref().unchecked_ref()));
    mousedown.forget();

    // 鼠标弹起
    let mouseup = Closure::wrap(Box::new(move |e: MouseEvent| {
        e.prevent_default();
        e.stop_propagation();
        let btn = e.button();
        console_log!("mouseup {}", btn);
    }) as Box<dyn FnMut(MouseEvent)> );
    canvas.set_onmouseup(Some(mouseup.as_ref().unchecked_ref()));
    mouseup.forget();

    // 键盘按下
    let keydown = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        e.prevent_default();
        e.stop_propagation();
        let code = e.key_code();
        console_log!("keydown {}", code);
    }) as Box<dyn FnMut(KeyboardEvent)>);
    canvas.set_onkeydown(Some(keydown.as_ref().unchecked_ref()));
    keydown.forget();

    // 键盘弹起
    let keyup = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        e.prevent_default();
        e.stop_propagation();
        let code = e.key_code();
        console_log!("keyup {}", code);
    }) as Box<dyn FnMut(KeyboardEvent)>);
    canvas.set_onkeyup(Some(keyup.as_ref().unchecked_ref()));
    keyup.forget();


    Ok(ws)
}