const path = require('path');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');

const distPath = path.resolve(__dirname, "dist");
module.exports = (env, argv) => {
    return {
        devServer: {
            historyApiFallback: {
                index:'/'
            },
            contentBase: distPath,
            compress: argv.mode === 'production',
            port: 8000,
            proxy: {
                '/api': 'http://localhost:3030'
            }
        },
        entry: './bootstrap.js',
        output: {
            path: distPath,
            filename: "tanoshi-web.js",
            webassemblyModuleFilename: "tanoshi-web.wasm",
            publicPath: "/"
        },
        plugins: [
            new CopyWebpackPlugin([
                { from: './static', to: distPath }
            ]),
            new WasmPackPlugin({
                crateDirectory: ".",
                extraArgs: "--no-typescript",
            })
        ],
        watch: argv.mode !== 'production'
    };
};