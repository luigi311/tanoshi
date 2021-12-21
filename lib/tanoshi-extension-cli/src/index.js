import minimist from 'minimist';
import build from './cmds/build.js';
import test from './cmds/test.js';

export default function () {
    const args = minimist(process.argv.slice(2))
    const cmd = args._[0]
    switch (cmd) {
        case 'build':
            build();
            break;
        case 'test':
            test(args._[1] === '--nocapture');
            break;
        default:
            console.error(`"${cmd}" is not a valid command!`)
            break
    }
}