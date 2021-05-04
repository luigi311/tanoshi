module.exports = {
    theme: {
        extend: {
            colors: {
                'accent': '#5b749b',
                'accent-lighter': '#7e93b3',
                'accent-darker': '#455876'
            },
            height: {
                page: 'calc(100vw * 1.59)',
                '1/2': '50%',
            },
            spacing: {
                '7/5': '141.5094339622642%',
            },
        },
        container: {
            center: true,
        },
    },
    variants: {
        backgroundColor: ['dark', 'hover', 'focus', 'disabled'],
        textColor: ['dark', 'hover', 'focus', 'disabled'],
    },
    plugins: [],
    darkMode: 'media'
}
