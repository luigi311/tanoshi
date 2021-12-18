import { Parcel } from '@parcel/core';
import fs from "fs";
import path from "path";

export default async () => {
    let files = fs.readdirSync('./src', { withFileTypes: true });
    let modules = [];
    for (var file of files) {
        if (file.name == 'index.ts') {
            modules.push({
                entries: `./src/index.ts`,
                filename: path.basename(path.resolve())
            });
        } else if (file.isDirectory()) {
            modules.push({
                entries: `./src/${file.name}/index.ts`,
                filename: file.name
            });
        }
    }

    for (var module of modules) {
        let bundler = new Parcel({
            entries: module.entries,
            defaultConfig: '@parcel/config-default',
            cacheDir: '../../.parcel-cache',
            defaultTargetOptions: {
                sourceMaps: false,
                outputFormat: "esmodule",
                isLibrary: true,
                shouldScopeHoist: true,
                distDir: "../../dist"
            },
            targets: {
                [file]: {
                    distDir: "../../dist",
                    distEntry: `${module.filename}.mjs`,
                    includeNodeModules: true,
                    sourceMap: false,
                    context: "node",
                    outputFormat: "esmodule",
                    isLibrary: true,
                    scopeHoist: true
                }
            }
        });

        try {
            let { bundleGraph, buildTime } = await bundler.run();
            let bundles = bundleGraph.getBundles();
            for (var bundle of bundles) {
                console.log(`✨ Built ${bundle.name} bundles in ${buildTime}ms!`);
                let dist = path.join(path.resolve('..', '..'), 'dist', bundle.name);
                if (process.platform === "win32") {
                    dist = `file://${dist}`;
                }
                console.log(dist);
                import(dist).then((module) => {
                    let source = new module.default();
                    console.log(JSON.stringify(source));
                });
            }
        } catch (err) {
            console.log(err.diagnostics);
        }
    }
}
