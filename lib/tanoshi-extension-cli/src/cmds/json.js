import fs from 'fs';
import path from "path";

export default async (dist_path) => {
    let extensions = []
    let files = fs.readdirSync(dist_path, { withFileTypes: true })
    for (const file of files) {
        if (file.name.endsWith('.mjs')) {
            let dist = path.join(path.resolve(dist_path), file.name)
            if (process.platform === "win32") {
                dist = `file://${dist}`;
            }
            // console.log(dist);
            await import(dist).then((module) => {
                let source = new module.default();
                // console.log(JSON.stringify(source));
                extensions.push({
                    "id": source.id,
                    "name": source.name,
                    "url": source.url,
                    "version": source.version,
                    "icon": source.icon,
                    "languages": source.languages,
                    "nsfw": source.nsfw,
                });
            });
        }
    }

    extensions.sort((a, b) => {
        if (a.id > b.id) {
            return 1
        }
        if (a.id < b.id) {
            return -1
        }
        return 0;
    });

    fs.writeFileSync(path.join(dist_path, 'index.json'), JSON.stringify(extensions));
}