/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        neon: {
          50: '#ebf5ff',
          100: '#d6ebff',
          200: '#a8d8ff',
          300: '#66bfff',
          400: '#1a9fff',
          500: '#0080ff',
          600: '#0060cc',
          700: '#004499',
          800: '#003380',
          900: '#002266',
          950: '#001144',
        },
        cyber: {
          50: '#f0e6ff',
          100: '#d9b3ff',
          200: '#bf80ff',
          300: '#a64dff',
          400: '#8c1aff',
          500: '#7300e6',
          600: '#5c00b3',
          700: '#450080',
          800: '#2e004d',
          900: '#17001a',
          950: '#0a000d',
        },
        surface: {
          50: '#f0f4ff',
          100: '#d0d8e8',
          200: '#a0b0cc',
          300: '#7088a8',
          400: '#506888',
          500: '#354868',
          600: '#253048',
          700: '#1a2030',
          800: '#121820',
          900: '#0a0e14',
          950: '#05080c',
        }
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'Avenir', 'Helvetica', 'Arial', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
      },
      animation: {
        'fade-in': 'fadeIn 0.3s ease-out',
        'slide-up': 'slideUp 0.4s ease-out',
        'pulse-glow': 'pulseGlow 1.5s infinite',
        'spin-slow': 'spin 3s linear infinite',
        'glow-pulse': 'glowPulse 2s ease-in-out infinite',
        'float': 'float 6s ease-in-out infinite',
        'scan-line': 'scanLine 4s linear infinite',
        'grid-move': 'gridMove 20s linear infinite',
        'twinkle': 'twinkle 3s ease-in-out infinite',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(10px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        pulseGlow: {
          '0%, 100%': { opacity: '1', transform: 'scale(1)' },
          '50%': { opacity: '0.85', transform: 'scale(0.98)' },
        },
        glowPulse: {
          '0%, 100%': { boxShadow: '0 0 20px rgba(0, 128, 255, 0.3)' },
          '50%': { boxShadow: '0 0 40px rgba(0, 128, 255, 0.6)' },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-20px)' },
        },
        scanLine: {
          '0%': { transform: 'translateY(-100%)' },
          '100%': { transform: 'translateY(100vh)' },
        },
        gridMove: {
          '0%': { transform: 'translateY(0)' },
          '100%': { transform: 'translateY(40px)' },
        },
        twinkle: {
          '0%, 100%': { opacity: '0.2' },
          '50%': { opacity: '1' },
        },
      }
    },
  },
  plugins: [],
}
