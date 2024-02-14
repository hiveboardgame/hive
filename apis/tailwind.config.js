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
        'board-xxl': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(110px, 1fr))',
        'board-lg': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(150px, 1fr))',
        'board-sm': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(120px, 1fr))',
        'board-xs': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(105px, 1fr))',
      },
      colors: {
        transparent: 'transparent',
        current: 'currentColor',
        'hive-black': '#3a3a3a',
        'hive-white': '#f0ead6',
        'li-hover-dark': '#999',
        'li-hover-light': '#787878',
        'light': '#edebe9',
        'odd-light': '#f7f6f5',
        'even-light': '#ffffff',
        'dark': '#161512',
        'odd-dark': '#302E2C',
        'even-dark': '#262421',
        'hover-blue-light': '#1b78d0',
        'hover-blue-dark': '#3692e7',
        'blue-light': '#d1e4f6',
        'blue-dark': '#293a49',
        'ant-blue': '#3574a5',
        'pillbug-teal': '#40afa1',
        'ladybug-red': '#d61a35',
        'grasshopper-green': '#3f9b3a',
        'queen-orange': '#f68c11',
      },
      dropShadow: {
        'w': '1px 1px 1px #888888',
        'b': '1px 1px 1px #888888',
      }
    },
  },
  plugins: [],
}
