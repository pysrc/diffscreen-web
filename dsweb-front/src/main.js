import init, { start_websocket } from "../pkg/dsweb_front"

var ws = null;

const start = (canvasid, url) => {    
    init().then(()=>{
        ws = start_websocket("canvas", "ws://127.0.0.1:41290");
    });
}

const stop = () => {
    ws.close();
}

export {
    start,
    stop
};