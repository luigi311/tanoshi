module.exports = {
    theme: {
        extend: {
            colors: {
                'tachiyomi-blue': '#5b749b',
                'tachiyomi-blue-lighter': '#7e93b3',
                'tachiyomi-blue-darker': '#455876'
            },
            minHeight: {
                '24': '6rem'
            },
            height: {
                page: 'calc(100vw * 1.59)',
                '1/2': '50%'
            },
            boxShadow: {
                top: '0 -1px 3px 0px rgba(0,0,0,0.1), 0 -1px 2px 0 rgba(0, 0, 0, .06)'
            }
        },
        container: {
            center: true,
        },
    },
    variants: {
        backgroundColor: ['responsive', 'hover', 'focus', 'active', 'group-hover', 'disabled'],
        tableLayout: ['responsive', 'hover', 'focus'],
    },
    plugins: [],
}
