# Development Server

The `serve` command provides a local development server with hot reloading for rapid iteration during site development.

## Basic Usage

```bash
# Start server on default port (3000)
yew-ssg serve

# Start with custom port
yew-ssg serve --port 8080

# Start and open browser automatically
yew-ssg serve --open

# Serve from a different directory
yew-ssg serve --site-dir /path/to/site
```

## Command Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--port` | `-p` | 3000 | Port to listen on |
| `--site-dir` | `-s` | Current directory | Site directory to serve |
| `--open` | `-o` | false | Open browser automatically |

## Features

### Hot Reloading

The development server automatically watches for file changes and triggers a rebuild:

- **Content files** (`.md` in `content/`): Pages and sections
- **Templates** (`.html` in `templates/`): Tera template files
- **Styles** (`.scss`/`.sass` in `styles/`): SCSS stylesheets
- **Static files** (`static/`): Images, fonts, and other assets
- **Configuration** (`site.toml`): Site configuration changes

When a change is detected, the server rebuilds the site and sends a reload signal to connected browsers via WebSocket.

### Live Reload Protocol

The server uses WebSocket for instant browser refresh:

1. Server starts on the specified port
2. HTML pages are injected with a live reload script
3. Browser connects to `/__ws__` WebSocket endpoint
4. On file change, server broadcasts reload message
5. Browser refreshes automatically

### Error Overlay

When a build fails, an error overlay is displayed in the browser showing:

- Error type and message
- File that caused the error
- Suggested fixes (when available)

The overlay automatically dismisses when the error is resolved.

### Graceful Shutdown

Press `Ctrl+C` to gracefully shut down the server:

- In-flight requests complete
- WebSocket connections close cleanly
- Build operations are cancelled safely

## Architecture

The development server is built on:

- **axum 0.8**: Web framework built on hyper
- **tokio**: Async runtime
- **notify**: Cross-platform file system watching
- **tower-http**: Static file serving with proper Content-Type headers

### Module Structure

```
generator/src/serve/
├── mod.rs         # Module exports
├── error.rs       # ServeError types
├── websocket.rs   # WebSocket message types
├── watcher.rs     # File watching and change categorization
├── injector.rs    # HTML injection for live reload script
└── server.rs      # DevServer implementation
```

### Key Types

- [`DevServer`](../api-reference.md): Main server configuration and startup
- [`ServeError`](../api-reference.md): Error types for server operations
- [`ChangeType`](../api-reference.md): Categorization of file changes
- [`WebSocketMessage`](../api-reference.md): WebSocket protocol messages

## Workflow Integration

### After `init`

The recommended workflow after creating a new site:

```bash
# Create a new site
yew-ssg init my-site

# Navigate to the site
cd my-site

# Start development server with browser auto-open
yew-ssg serve --open
```

### With `build`

The serve command runs `build` internally. You don't need to run build separately during development. However, for production builds:

```bash
# Development: use serve
yew-ssg serve

# Production: use build
yew-ssg build --release
```

## Troubleshooting

### Port Already in Use

If port 3000 is already in use:

```bash
yew-ssg serve --port 3001
```

Or check what's using the port:

```bash
# Linux/macOS
lsof -i :3000

# Windows
netstat -ano | findstr :3000
```

### Files Not Being Watched

Ensure files are in the correct directories:

- Content: `content/` directory with `.md` extension
- Templates: `templates/` directory with `.html` extension
- Styles: `styles/` directory with `.scss` or `.sass` extension
- Static: `static/` directory

### Browser Not Refreshing

1. Check WebSocket connection in browser dev tools (Network tab → WS)
2. Ensure JavaScript is enabled
3. Check for console errors
4. Verify the live reload script is injected (view page source)

### Build Errors

Build errors are displayed in both the terminal and browser overlay. Common issues:

- **Missing template**: Check template file exists in `templates/`
- **Invalid frontmatter**: Validate TOML syntax in `+++` blocks
- **SCSS syntax error**: Check for missing semicolons or braces
