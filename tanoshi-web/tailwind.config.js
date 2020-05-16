module.exports = {
    theme: {
        extend: {
            colors: {
                'tachiyomi-blue': '#5b749b',
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