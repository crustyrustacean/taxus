# TypeScript Assets

Taxus can compile TypeScript into browser-native JavaScript modules during a
site build. This is useful for small interactive behaviors—theme toggles,
menus, filters, calculators, galleries and client-side API calls—without
introducing Node.js, npm, Vite or a JavaScript bundler.

TypeScript runs in the visitor's browser only after the generated page loads.
Taxus performs the type-check and compilation ahead of time; the deployed site
contains ordinary JavaScript modules and optional source maps.

## Start a TypeScript site

Released Taxus binaries include the native TypeScript compiler for their target
platform. Initialize a site with the TypeScript scaffold:

```bash
taxus init my-site --no-islands --typescript
cd my-site
taxus build
taxus serve --open
```

The scaffold contains:

```text
scripts/
├── greeting.ts       # imported example module
└── main.ts           # browser entrypoint
tsconfig.json
```

`main.ts` uses a browser-resolvable import:

```ts
import { greeting } from "./greeting.js";

document.body.dataset.taxusGreeting = greeting;
```

The `.js` extension is intentional. TypeScript's bundler resolution uses it to
find `greeting.ts` while checking, and the emitted browser module keeps the
`.js` specifier.

## Configuration

The scaffold writes:

```toml
[typescript]
enabled = true
```

and a `tsconfig.json` with strict checking, browser libraries, ES modules and
source maps enabled. Place additional `.ts` files under `scripts/`; the
`include` pattern picks them up automatically.

To enable source maps in an existing site:

```json
{
  "compilerOptions": {
    "sourceMap": true
  }
}
```

Source maps are written beside the generated modules under `dist/static/ts/`
and let browser developer tools show the original TypeScript source.

## Compiler availability

Compiler inclusion is a Cargo build-time choice:

- Published Taxus binaries include a native compiler for macOS arm64/x64,
  Linux arm64/x64 and Windows arm64/x64.
- A normal source build does not include compiler archives.
- Build Taxus with `cargo build --features typescript` to include the matching
  compiler in a locally built binary.
- A compiler-free binary can use an installed compiler by setting:

  ```toml
  [typescript]
  enabled = true
  compiler = "/path/to/tsc"
  ```

If a site enables TypeScript but the binary has neither a bundled compiler nor a
configured `compiler`, Taxus reports that requirement before attempting the
build.

## Scope and limits

Taxus compiles TypeScript; it does not bundle npm packages. Use relative
ES-module imports that browsers can resolve directly. Package resolution,
tree-shaking, minification and bundling are outside this feature's scope.

For a small browser behavior, TypeScript is usually the right tool. Use a Yew
WASM island when the component needs complex reactive state, substantial
computation or the Yew component model. Use ordinary `static/scripts.js` when
plain JavaScript is sufficient and type checking is not valuable.

## Build and serve behavior

During `taxus build`, TypeScript is checked before the generated module
directory replaces the previous `dist/static/ts/` directory. If checking or
emission fails, the previous successful output remains intact.

`taxus serve` watches both `scripts/` and `tsconfig.json`. A TypeScript edit
triggers a rebuild and browser reload. A failed rebuild is reported like any
other build failure; after the source is fixed, the next rebuild recovers.

For the complete configuration reference, see the
[`[typescript]` section](./configuration.md#typescript-section). For the
browser/runtime mental model, see [Islands Architecture](./islands.md).
