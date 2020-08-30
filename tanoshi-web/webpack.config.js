const path = require('path');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const WorkboxPlugin = require('workbox-webpack-plugin');

const distPath = path.resolve(__dirname, "dist");
module.exports = (env, argv) => {
    const isProduction = (argv.mode === 'production');//package.json scripts -> build

    return {
        devServer: {
            historyApiFallback: {
                index: '/'
            },
            contentBase: distPath,
            host: '0.0.0.0',
            port: 8000,
            proxy: {
                '/api': 'http://127.0.0.1:3030'
            }
        },
        entry: './index.js',
        output: {
            path: distPath,
            publicPath: '/',
            filename: isProduction ? '[name].[contenthash].js' : '[name].[hash].js'
        },
        optimization: {
            runtimeChunk: 'single',
            splitChunks: {
                cacheGroups: {
                    vendor: {
                        test: /[\\/]node_modules[\\/]/,
                        name: 'vendors',
                        chunks: 'all',
                    },
                },
            },
        },
        plugins: [
            new CleanWebpackPlugin(),
            new HtmlWebpackPlugin({
                template: 'static/index.html'
            }),
            new CopyWebpackPlugin([
                { from: 'static', to: distPath }
            ]),
            new WasmPackPlugin({
                crateDirectory: ".",
                extraArgs: "--no-typescript",
            }),
            new WorkboxPlugin.GenerateSW({
                // these options encourage the ServiceWorkers to get in there fast
                // and not allow any straggling "old" SWs to hang around
                clientsClaim: true,
                skipWaiting: true,
            }),
        ],
        watch: argv.mode !== "production",
        module: {
            rules: [
                {
                    test: /\.css$/,
                    exclude: /node_modules/,
                    use: [
                        {
                            loader: 'style-loader',
                        },
                        {
                            loader: 'css-loader',
                            options: {
                                importLoaders: 1,
                            }
                        },
                        {
                            loader: 'postcss-loader'
                        }
                    ]
                }
            ]
        }
    };
};