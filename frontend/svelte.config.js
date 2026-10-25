import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  kit: {
    // SPA: a single index.html fallback, served from the Rust binary via
    // rust-embed (TDD section 3/4). Output goes to ../crates/.../assets? No:
    // keep build output here; the native crate embeds `frontend/build`.
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      precompress: false,
      strict: false
    })
  }
};

export default config;
