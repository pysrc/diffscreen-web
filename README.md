## Diffscreen-web

使用wasm技术，实现客户端无感跨端显示

## 编译

在root目录下可以编译dsweb-core，这个是被控端需要运行的程序

`cargo build --release`

然后切换到dsweb-front，编译前端工程，参照里面的README

## 第三方库依赖安装参考（主要是linux）

* https://github.com/quadrupleslap/scrap
* https://github.com/enigo-rs/enigo

## 计划中的功能

* 文件传输
* 移动端键盘鼠标适配
* webrtc协议转换
