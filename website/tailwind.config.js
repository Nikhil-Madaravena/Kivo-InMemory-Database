/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'brand-bg': '#0B0B14',
        'brand-primary': '#8A2BE2',
        'brand-secondary': '#4169E1',
        'brand-tertiary': '#00d2ff',
        'brand-text': '#F8F9FA',
        'brand-muted': '#A0AABF',
      },
      fontFamily: {
        sans: ['Inter', 'sans-serif'],
        mono: ['"Fira Code"', 'monospace'],
      },
      animation: {
        'float': 'float 20s infinite ease-in-out alternate',
        'float-reverse': 'float-reverse 25s infinite ease-in-out alternate',
        'float-fast': 'float 18s infinite ease-in-out alternate-reverse',
        'fade-in-up': 'fadeInUp 0.8s forwards cubic-bezier(0.2, 0.8, 0.2, 1)',
        'type-terminal': 'typeTerminal 0.3s ease-out forwards',
        'blink': 'blink 1s step-end infinite',
      },
      keyframes: {
        float: {
          '0%': { transform: 'translate(0, 0) scale(1)' },
          '50%': { transform: 'translate(50px, 50px) scale(1.1)' },
          '100%': { transform: 'translate(-50px, 20px) scale(0.9)' },
        },
        'float-reverse': {
          '0%': { transform: 'translate(0, 0) scale(1)' },
          '50%': { transform: 'translate(-40px, -30px) scale(1.15)' },
          '100%': { transform: 'translate(30px, -50px) scale(0.95)' },
        },
        fadeInUp: {
          'to': { opacity: '1', transform: 'translateY(0)' },
        },
        typeTerminal: {
          'from': { opacity: '0', transform: 'translateY(5px)' },
          'to': { opacity: '1', transform: 'translateY(0)' },
        },
        blink: {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0' },
        }
      }
    },
  },
  plugins: [],
}
