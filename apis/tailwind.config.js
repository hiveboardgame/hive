/** @type {import('tailwindcss').Config} */
const defaultTheme = require('tailwindcss/defaultTheme')

module.exports = {
  future: {
    hoverOnlyWhenSupported: true,
  },
  darkMode: 'class',
  content: {
    relative: true,
    files: ["*.html", "./src/**/*.rs"],
  },
  theme: {
    screens: {
      'xs': '360px',
      ...defaultTheme.screens,
    },
    extend: {
      screens: { 'short': { 'raw': '(max-height: 700px)' }, },
      gridTemplateColumns: {
        'board-xl': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(0, 220px))',
        'board': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(170px, 1fr))',
      },
      colors: {
        transparent: 'transparent',
        current: 'currentColor',
        'board-twilight': '#47545a',
        'board-dawn': '#edebe9',
        'reserve-twilight': '#212836',
        'reserve-dawn': '#edebe9',
        'header-twilight': '#2e3e48',
        'header-dawn': '#d1d5db',
        'button-twilight': '#7da1b2',
        'button-dawn': '#3574a5',
        'orange-twilight': '#e9ac43',
        'orange-dawn': '#f68c11',
        'hive-black': '#3a3a3a',
        'hive-white': '#f0ead6',
        'light': '#edebe9',
        'odd-light': '#f7f6f5',
        'even-light': '#ffffff',
        'odd-dark': '#302E2C',
        'even-dark': '#262421',
        'pillbug-teal': '#40afa1',
        'ladybug-red': '#d61a35',
        'grasshopper-green': '#3f9b3a',
        'blue-light': '#d1e4f6',
        'blue-dark': '#293a49',
      },
      dropShadow: {
        'w': '1px 1px 1px #888888',
        'b': '1px 1px 1px #888888',
      }
    },
  },
  plugins: [
    require('@tailwindcss/typography')
  ],
}
