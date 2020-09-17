module.exports = {
    theme: {
        extend: {
            colors: {
                'accent': '#5b749b',
                'accent-lighter': '#7e93b3',
                'accent-darker': '#455876'
            },
            minHeight: {
                '24': '6rem'
            },
            height: {
                page: 'calc(100vw * 1.59)',
                '1/2': '50%',
            },
            padding: {
                '7/5': '141.5094339622642%',
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
        backgroundColor: ['dark', 'responsive', 'hover', 'focus', 'active', 'disabled'],
        tableLayout: ['responsive', 'hover', 'focus'],
        textColor: ['dark', 'responsive', 'hover', 'focus', 'active', 'group-hover', 'disabled'],
    },
    plugins: [],
    dark: 'class',
    experimental: {
        darkModeVariant: true,
    }
}
