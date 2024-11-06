import { fileURLToPath, URL } from 'node:url'

import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import vueDevTools from 'vite-plugin-vue-devtools'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
    const env = loadEnv(mode, process.cwd());
    return {
        plugins: [
            vue({
                template: {
                    compilerOptions: {
                        // treat all tags with a dash as custom elements
                        //isCustomElement: (tag) => tag.includes('-')
                    },
                },
            }),
            vueJsx(),
            vueDevTools(),
        ],
        resolve: {
            alias: {
                '@': fileURLToPath(new URL('./src', import.meta.url)),
            },
        },
        server: {
            proxy: {
                '/api': {
                    target: 'http://localhost:6734',
                    changeOrigin: true,
                    //rewrite: path => path.replace(/^\/api/, ''),
                },
                "/socket.io": {
                    target: "http://localhost:6734",
                    changeOrigin: true,
                    ws: true,
                },
            },
        },
        base: env.VITE_PUBLIC_PATH || '/',
    }
})
