/** @type {import('tailwindcss').Config} */
export default {
    content: [
        "./index.html",
        "./src/**/*.{js,ts,jsx,tsx}",
    ],
    theme: {
        extend: {
            fontFamily: {
                sans: ['Inter', 'system-ui', 'sans-serif'],
            },
            colors: {
                background: "#030308",
                surface: "#0f0f16",
                "surface-highlight": "#1a1a24",
                primary: "#00f2ea",
                secondary: "#bd00ff",
                danger: "#ff0055",
                success: "#00ff9d",
                text: {
                    main: "#ffffff",
                    muted: "#9494a0"
                }
            },
            animation: {
                'pulse-glow': 'pulse-glow 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
                'float': 'float 6s ease-in-out infinite',
            },
            keyframes: {
                'pulse-glow': {
                    '0%, 100%': { opacity: '0.4', filter: 'blur(8px)' },
                    '50%': { opacity: '1', filter: 'blur(12px)' },
                },
                'float': {
                    '0%, 100%': { transform: 'translateY(0)' },
                    '50%': { transform: 'translateY(-10px)' },
                }
            },
            backdropBlur: {
                xs: '2px',
            }
        },
    },
    plugins: [],
}
