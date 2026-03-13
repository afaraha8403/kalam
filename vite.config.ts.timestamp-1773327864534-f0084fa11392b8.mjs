// vite.config.ts
import { defineConfig } from "file:///C:/Users/alifa/Development/BalaHeadache%20Projects/kalam/node_modules/vite/dist/node/index.js";
import { svelte } from "file:///C:/Users/alifa/Development/BalaHeadache%20Projects/kalam/node_modules/@sveltejs/vite-plugin-svelte/src/index.js";
import { sveltePreprocess } from "file:///C:/Users/alifa/Development/BalaHeadache%20Projects/kalam/node_modules/svelte-preprocess/dist/index.js";
var vite_config_default = defineConfig(async () => ({
  plugins: [
    svelte({
      preprocess: sveltePreprocess()
    })
  ],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"]
    }
  },
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.{test,spec}.{ts,tsx}"]
  }
}));
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcudHMiXSwKICAic291cmNlc0NvbnRlbnQiOiBbImNvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9kaXJuYW1lID0gXCJDOlxcXFxVc2Vyc1xcXFxhbGlmYVxcXFxEZXZlbG9wbWVudFxcXFxCYWxhSGVhZGFjaGUgUHJvamVjdHNcXFxca2FsYW1cIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfZmlsZW5hbWUgPSBcIkM6XFxcXFVzZXJzXFxcXGFsaWZhXFxcXERldmVsb3BtZW50XFxcXEJhbGFIZWFkYWNoZSBQcm9qZWN0c1xcXFxrYWxhbVxcXFx2aXRlLmNvbmZpZy50c1wiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9pbXBvcnRfbWV0YV91cmwgPSBcImZpbGU6Ly8vQzovVXNlcnMvYWxpZmEvRGV2ZWxvcG1lbnQvQmFsYUhlYWRhY2hlJTIwUHJvamVjdHMva2FsYW0vdml0ZS5jb25maWcudHNcIjtpbXBvcnQgeyBkZWZpbmVDb25maWcgfSBmcm9tICd2aXRlJ1xuaW1wb3J0IHsgc3ZlbHRlIH0gZnJvbSAnQHN2ZWx0ZWpzL3ZpdGUtcGx1Z2luLXN2ZWx0ZSdcbmltcG9ydCB7IHN2ZWx0ZVByZXByb2Nlc3MgfSBmcm9tICdzdmVsdGUtcHJlcHJvY2VzcydcblxuZXhwb3J0IGRlZmF1bHQgZGVmaW5lQ29uZmlnKGFzeW5jICgpID0+ICh7XG4gIHBsdWdpbnM6IFtcbiAgICBzdmVsdGUoe1xuICAgICAgcHJlcHJvY2Vzczogc3ZlbHRlUHJlcHJvY2VzcygpLFxuICAgIH0pXG4gIF0sXG4gIGNsZWFyU2NyZWVuOiBmYWxzZSxcbiAgc2VydmVyOiB7XG4gICAgcG9ydDogNTE3MyxcbiAgICBzdHJpY3RQb3J0OiB0cnVlLFxuICAgIHdhdGNoOiB7XG4gICAgICBpZ25vcmVkOiBbXCIqKi9zcmMtdGF1cmkvKipcIl0sXG4gICAgfSxcbiAgfSxcbiAgdGVzdDoge1xuICAgIGVudmlyb25tZW50OiAnanNkb20nLFxuICAgIGdsb2JhbHM6IHRydWUsXG4gICAgaW5jbHVkZTogWydzcmMvKiovKi57dGVzdCxzcGVjfS57dHMsdHN4fSddLFxuICB9LFxufSkpXG4iXSwKICAibWFwcGluZ3MiOiAiO0FBQWtXLFNBQVMsb0JBQW9CO0FBQy9YLFNBQVMsY0FBYztBQUN2QixTQUFTLHdCQUF3QjtBQUVqQyxJQUFPLHNCQUFRLGFBQWEsYUFBYTtBQUFBLEVBQ3ZDLFNBQVM7QUFBQSxJQUNQLE9BQU87QUFBQSxNQUNMLFlBQVksaUJBQWlCO0FBQUEsSUFDL0IsQ0FBQztBQUFBLEVBQ0g7QUFBQSxFQUNBLGFBQWE7QUFBQSxFQUNiLFFBQVE7QUFBQSxJQUNOLE1BQU07QUFBQSxJQUNOLFlBQVk7QUFBQSxJQUNaLE9BQU87QUFBQSxNQUNMLFNBQVMsQ0FBQyxpQkFBaUI7QUFBQSxJQUM3QjtBQUFBLEVBQ0Y7QUFBQSxFQUNBLE1BQU07QUFBQSxJQUNKLGFBQWE7QUFBQSxJQUNiLFNBQVM7QUFBQSxJQUNULFNBQVMsQ0FBQywrQkFBK0I7QUFBQSxFQUMzQztBQUNGLEVBQUU7IiwKICAibmFtZXMiOiBbXQp9Cg==
