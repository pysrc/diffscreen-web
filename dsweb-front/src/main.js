import init, { start_websocket } from "../pkg/dsweb_front"

var ws = null;

const start = (canvasid, url) => {    
    init().then(()=>{
        ws = start_websocket(canvasid, url);
    });
}

const stop = () => {
    ws.close();
}

export {
    start,
    stop
};