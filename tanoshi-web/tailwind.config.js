const colors = require('tailwindcss/colors')

module.exports = {
    theme: {
        extend: {
            colors: {
                'accent': '#991B1B',
                'accent-lighter': '#B91C1C',
                'accent-darker': '#7F1D1D',
            },
            height: {
                page: 'calc(100vw * 1.59)',
                '1/2': '50%',
            },
            spacing: {
                '7/5': '141.5094339622642%',
            },
        },
        colors: {
            transparent: 'transparent',
            current: 'currentColor',
            black: colors.black,
            white: colors.white,
            gray: colors.trueGray,
            red: colors.red,
            yellow: colors.amber,
            blue: colors.blue
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
