## Diffscreen-web

使用wasm技术，实现客户端无感跨端显示

## 编译

在root目录下可以编译dsweb-core，这个是被控端需要运行的程序

`cargo build --release --workspace --exclude dsweb-front`

这里会编译出可执行文件：
`./target/release/dsweb-core`

然后切换到dsweb-front，编译前端工程，参照里面的README，然后将打包后的产物跟index.html放在当前public目录下，运行dsweb-core即可

产物结构如下
```
│ dsweb-core.exe
|
├─files
└─public
        29ff266c59292a226eee.wasm
        dsweb.js
        index.html
```

其中files文件夹是上传下载的中转文件夹，
访问地址（如果是本地）：http://127.0.0.1:41290

## Linux下如何通过ssh登入后显示桌面

首先需要有X11桌面环境，然后通过ssh进入后需要设置环境变量：
`export DISPLAY=:0`
然后就可以运行dsweb-core程序愉快远程了！

## 第三方库依赖安装参考（主要是linux）

* https://github.com/quadrupleslap/scrap
* https://github.com/enigo-rs/enigo
* https://github.com/aweinstock314/rust-clipboard

## Webrtc版本？

目前Webrtc版本暂不开源，商业联系邮箱1570184051@qq.com
