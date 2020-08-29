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
        backgroundColor: ['dark', 'responsive', 'hover', 'dark-hover', 'focus', 'active', 'group-hover', 'dark-group-hover', 'disabled', 'dark-disabled'],
        tableLayout: ['responsive', 'hover', 'focus'],
        textColor: ['dark', 'responsive', 'hover', 'dark-hover', 'focus', 'active', 'group-hover', 'disabled'],
        borderColor: ['dark'],
        height: ['responsive']
    },
    plugins: [
        require('tailwindcss-dark-mode')()
    ]
}
