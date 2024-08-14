
const CopyWebpackPlugin = require('copy-webpack-plugin');
var path = require("path")

module.exports = {
    // 入口
    entry: "./src/main.js",
    // 输出
    output: {
        // 输出路径，这里要用绝对路径
        path: path.resolve(__dirname, "dist"),
        // 输出文件名
        filename: "dsweb.js",
        library: 'dsweb',
        libraryTarget: 'umd'
    },
    mode: "production",
    plugins: [
        new CopyWebpackPlugin({
            patterns: [
                { from: path.resolve(__dirname, "public"), to: path.resolve(__dirname, "dist") },
            ],
        }),
    ],
}