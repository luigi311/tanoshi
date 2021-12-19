import minimist from 'minimist';
import build from './cmds/build.js';
import build_test from './cmds/build_test.js';

export default function () {
    const args = minimist(process.argv.slice(2))
    const cmd = args._[0]
    switch (cmd) {
        case 'build':
            build();
            break;
        case 'test':
            build_test();
            break;
        default:
            console.error(`"${cmd}" is not a valid command!`)
            break
    }
}