import minimist from 'minimist';
import build from './cmds/build.js';
import json from './cmds/json.js';
import test from './cmds/test.js';

export default function () {
    const args = minimist(process.argv.slice(2))
    const cmd = args._[0]
    switch (cmd) {
        case 'build':
            build();
            break;
        case 'test':
            test(args['nocapture']);
            break;
        case 'json':
            json(args['path'] ? args['path'] : './dist');
            break;
        default:
            console.error(`"${cmd}" is not a valid command!`)
            break
    }
}