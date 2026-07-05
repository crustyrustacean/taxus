# Development Server

The `serve` command provides a local development server with hot reloading.

## Basic Usage

```bash
# Start server on default port (3000)
taxus serve

# Start with custom port
taxus serve --port 8080

# Start and open browser automatically
taxus serve --open

# Serve from a different directory
taxus serve --dir ./my-site
```

The serve command performs an initial build automatically before starting the server.

## Command Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--port` | `-p` | 3000 | Port to listen on |
| `--verbose` | `-v` | false | Print detailed build progress |
| `--quiet` | `-q` | false | Suppress all output except errors |
| `--open` | `-o` | false | Open browser automatically |

## Features

### Hot Reloading

The server watches for file changes and triggers a rebuild:

- **Content files** (`.md` in `content/`)
- **Templates** (`.html` in `templates/`)
- **Styles** (`.scss`/`.sass` in `styles/`)
- **Static files** (`static/`)
- **Configuration** (`site.toml`)

When a change is detected, the server rebuilds and sends a reload signal to connected browsers via WebSocket.

### Live Reload Protocol

1. Server starts on the specified port
2. HTML pages are injected with a live reload script
3. Browser connects to `/__ws__` WebSocket endpoint
4. On file change, server broadcasts reload message
5. Browser refreshes automatically

### Error Overlay

Build errors are displayed in the browser with:
- Error type and message
- File that caused the error
- Suggested fixes (when available)

The overlay dismisses when the error is resolved.

### Graceful Shutdown

Press `Ctrl+C` to shut down cleanly:
- In-flight requests complete
- WebSocket connections close cleanly
- Build operations are cancelled safely

## Workflow

### After `init`

```bash
taxus init my-site
cd my-site
taxus serve --open
```

### With `build`

The serve command runs `build` internally. For production:

```bash
# Development
taxus serve

# Production
taxus build
```

## Troubleshooting

### Port Already in Use

```bash
taxus serve --port 3001
```

Check what's using the port:

```bash
# Linux/macOS
lsof -i :3000

# Windows
netstat -ano | findstr :3000
```

### Files Not Being Watched

Ensure files are in correct directories:
- Content: `content/` with `.md` extension
- Templates: `templates/` with `.html` extension
- Styles: `styles/` with `.scss` or `.sass`
- Static: `static/`

### Browser Not Refreshing

1. Check WebSocket connection in dev tools (Network → WS)
2. Ensure JavaScript is enabled
3. Check for console errors
4. Verify live reload script is injected (view source)
