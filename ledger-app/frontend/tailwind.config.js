/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        primary: {
          50:  '#eef9f7',
          100: '#d5f2ee',
          200: '#aee5de',
          400: '#34c5b3',
          500: '#14b8a6',
          600: '#0d9488',
          700: '#0f766e',
          800: '#115e59',
        },
      },
      boxShadow: {
        soft:  '0 1px 3px 0 rgba(0,0,0,0.07), 0 1px 2px -1px rgba(0,0,0,0.05)',
        card:  '0 1px 3px 0 rgba(0,0,0,0.07), 0 1px 2px -1px rgba(0,0,0,0.05)',
        hover: '0 4px 14px 0 rgba(0,0,0,0.10)',
        glow:  '0 0 0 3px rgba(20,184,166,0.25)',
      },
    }
  },
  plugins: []
};
