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
      colors: {
        transparent: 'transparent',
        current: 'currentColor',
        'li-green': '#629924',
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
        'b': '0.3px 0.3px 0.3px rgba(255, 255, 255, 1)'
      }
    },
  },
  plugins: [],
}