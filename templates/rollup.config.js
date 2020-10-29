import serve from 'rollup-plugin-serve';
import liveReload from 'rollup-plugin-livereload';
import { terser } from "rollup-plugin-terser";

const production = !process.env.ROLLUP_WATCH;

export default {
    input: "dist/js/bootstrap.js",
    output: {
        file: "dist/js/index.js",
        format: "iife",
        sourcemap: true,
    },
    plugins: [
        !production && serve({
            contentBase: "dist",
            verbose: true,
            open: true,
        }),
        !production && liveReload({
            watch: "dist"
        }),
        production && terser(),
    ],
};