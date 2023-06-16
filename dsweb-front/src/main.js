import init, { start_websocket } from "../pkg/dsweb_front"

var ws = null;
var transfer_ws = null;
var transferdom = null;

const start = (canvasid, transferid, url) => {    
    init().then(()=>{
        ws = start_websocket(canvasid, url);
        transferdom = document.getElementById(transferid);
        transfer_ws = new WebSocket(url, "diffscreen-transfer");
        transfer_ws.onopen = function(event) {
            console.log('Transfer Websocket 已经连接');
        };
        transfer_ws.onmessage = function(event) {
            var d = event.data;
            if(d.startsWith("copy-text")) {
                d = d.replace("copy-text ", "");
                transferdom.value = d;
            }
        };
    });
}

const stop = () => {
    ws.close();
    transfer_ws.close();
}

const paste_text = () => {
    var msg = transferdom.value;
    transfer_ws.send(`paste-text ${msg}`);
};

const copy_text = () => {
    transfer_ws.send("copy-text");
}

export {
    start,
    stop,
    paste_text,
    copy_text,
};