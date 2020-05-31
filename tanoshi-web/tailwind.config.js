module.exports = {
    theme: {
        extend: {
            colors: {
                'tachiyomi-blue': '#5b749b',
                'tachiyomi-blue-lighter': '#7e93b3',
                'tachiyomi-blue-darker': '#455876'
            }
        },
        container: {
            center: true,
        },
        minHeight: {
            '24': '6rem'
        }
    },
    variants: {
        backgroundColor: ['responsive', 'hover', 'focus', 'active', 'group-hover'],
        tableLayout: ['responsive', 'hover', 'focus'],
    },
    plugins: [],
}