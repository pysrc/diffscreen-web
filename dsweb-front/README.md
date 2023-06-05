## 打包工具安装

`rustup target add wasm32-unknown-unknown`

[wasm-pack安装教程](https://rustwasm.github.io/wasm-pack/installer/)

`npm i webpack webpack-cli -D`

## 编译

`wasm-pack build --target web`

`npx webpack`

## 运行

打包完后，dist目录下就是可以植入html的js跟wasm程序，可以运行在Nginx或者其他Web静态服务器上，调用方式参考index.html
