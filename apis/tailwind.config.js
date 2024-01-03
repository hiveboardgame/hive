/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: 'class',
  content: {
    relative: true,
    files: ["*.html", "./src/**/*.rs"],
  },
  theme: {
    extend: {
      screens: {
        'short': { 'raw': '(max-height: 700px)' },
      },
      gridTemplateColumns: {
        'board-xxl': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(110px, 1fr))',
        'board-lg': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(150px, 1fr))',
        'board-sm': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(120px, 1fr))',
        'board-xs': 'repeat(8, minmax(0, 1fr)) repeat(2, minmax(105px, 1fr))',
      },
      colors: {
        transparent: 'transparent',
        current: 'currentColor',
        'hive-white': '#3a3a3a',
        'hive-black': '#f0ead6',
        'li-green': '#629924',
        'li-hover-dark': '#999',
        'li-hover-light': '#787878',
        'li-red': '#c33',
        'light': '#edebe9',
        'odd-light': '#f7f6f5',
        'even-light': '#ffffff',
        'dark': '#161512',
        'odd-dark': '#302E2C',
        'even-dark': '#262421',
        'hover-blue-light': "#1b78d0",
        'hover-blue-dark': "#3692e7",
        'blue-light': "#d1e4f6",
        'blue-dark': "#293a49",


      },
      dropShadow: {
        'w': '0.3px 0.3px 0.3px rgba(0, 0, 0, 1)',
        'b': '0.3px 0.3px 0.3px rgba(255, 255, 255, 1)',
      }
    },
  },
  plugins: [],
}