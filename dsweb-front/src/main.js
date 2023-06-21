import init, { start_websocket } from "../pkg/dsweb_front"

var ws = null;
var transfer_ws = null;
var transferdom = null;

const start = (canvasid, transferid) => {
    init().then(()=>{
        ws = start_websocket(canvasid, `ws://${location.host}/diffscreen`);
        transferdom = document.getElementById(transferid);
        transfer_ws = new WebSocket(`ws://${location.host}/diffscreen-cb`);
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

const file_list_refresh = (files_callback) => {
    fetch(`/list`).then(function(response) {
        if(response.ok) {
            response.json().then((txt)=>{
                files_callback(txt);
            });
        }
    });
}

const download = (filename) => {
    if(filename) {
        window.open(`/files/${filename}`);
    } else {
        alert("请刷新文件列表");
    }
}

const upload = (file, progress_callback) => {
    if(!file) {
        return;
    }
    const formData = new FormData();
    formData.append('file', file);
    const xhr = new XMLHttpRequest();
    xhr.open('POST', `/upload`, true);
    xhr.upload.addEventListener('progress', function(event) {
        const progress = event.loaded / event.total * 100;
        progress_callback(progress);
    });
    xhr.addEventListener('load', function(event) {
        console.log(`Server response: ${xhr.responseText}`);
    });
    xhr.send(formData);
}

export {
    start,
    stop,
    paste_text,
    copy_text,
    file_list_refresh,
    download,
    upload,
};