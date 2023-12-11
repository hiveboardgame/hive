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

      },
      dropShadow: {
        'w': '0.3px 0.3px 0.3px rgba(0, 0, 0, 1)',
        'b': '0.3px 0.3px 0.3px rgba(255, 255, 255, 1)'
      }
    },
  },
  plugins: [],
}