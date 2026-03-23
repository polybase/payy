/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        primary: 'rgb(var(--primary) / <alpha-value>)',
        background: 'rgb(var(--background) / <alpha-value>)',
        text: 'rgb(var(--text) / <alpha-value>)',
        'border-accent': 'rgb(var(--border-accent) / <alpha-value>)',
        'gray-900': 'rgb(var(--gray-900) / <alpha-value>)',
        'gray-800': 'rgb(var(--gray-800) / <alpha-value>)'
      },
      fontFamily: {
        sans: ['Steradian', 'system-ui', 'sans-serif']
      },
      boxShadow: {
        glow: '0 0 0 1px rgba(224, 255, 50, 0.35), 0 8px 30px rgba(0, 0, 0, 0.35)'
      }
    }
  },
  plugins: []
}
