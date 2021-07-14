const path = require('path');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');
const { CleanWebpackPlugin } = require('clean-webpack-plugin');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const WorkboxPlugin = require('workbox-webpack-plugin');
const CompressionPlugin = require('compression-webpack-plugin');

const distPath = path.resolve(__dirname, "dist");
module.exports = (env, argv) => {
    const isProduction = (argv.mode === 'production');//package.json scripts -> build
    const plugins = [
        new CleanWebpackPlugin(),
        new HtmlWebpackPlugin({
            template: 'static/index.html'
        }),
        new CopyWebpackPlugin({
            patterns: [
                { from: 'static', to: distPath }
            ]
        }),
        new WasmPackPlugin({
            crateDirectory: ".",
            extraArgs: "--no-typescript",
        })
    ];
    if (isProduction) {
        const productionPlugins = [
            new WorkboxPlugin.GenerateSW({
                 // these options encourage the ServiceWorkers to get in there fast
                 // and not allow any straggling "old" SWs to hang around
                 clientsClaim: true,
                 skipWaiting: true,
             }),
             new CompressionPlugin({
                 filename: '[path][base].gz',
                 algorithm: 'gzip',
                 test: /\.js$|\.wasm$/,
                 threshold: 10240,
                 minRatio: 0.8,
             }),
             new CompressionPlugin({
                 filename: '[path][base].br',
                 algorithm: 'brotliCompress',
                 test: /\.(js|wasm)$/,
                 compressionOptions: {
                   // zlib’s `level` option matches Brotli’s `BROTLI_PARAM_QUALITY` option.
                   level: 11,
                 },
                 threshold: 10240,
                 minRatio: 0.8,
                 deleteOriginalAssets: false,
             }),
         ];
         plugins.push(...productionPlugins);
    }
    return {
        devServer: {
            historyApiFallback: {
                index: '/'
            },
            contentBase: distPath,
            host: '0.0.0.0',
            port: 3000,
            proxy: {
                '/graphql': 'http://127.0.0.1:3030',
                '/image': 'http://127.0.0.1:3030'
            }
        },
        entry: './static/js/index.js',
        output: {
            path: distPath,
            publicPath: '/',
            filename: isProduction ? '[name].[contenthash].js' : '[name].[fullhash].js'
        },
        plugins: plugins,
        watch: argv.mode !== "production",
        experiments: {
            asyncWebAssembly: true,
        },
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