/** @type {import('tailwindcss').Config} */
module.exports = {
  content: { 
    files: ["*.html", "./app/src/**/*.rs"],
  },
  theme: {
    extend: {
      dropShadow: {
        'w': '0.3px 0.3px 0.3px rgba(0, 0, 0, 1)',
        'b': '0.3px 0.3px 0.3px rgba(255, 255, 255, 1)'
      }
    },
  },
  plugins: [],
}

