# szz

> 새짹짹  
> szz  
> 🐣📢📢

szz is a desktop launcher and web platform for Minecraft servers. szz lets players join multiplayer servers with a single click, without manually enter server address, download specific client versions, or install require mods one by one.


## Developement Guide

### Prerequisites

- **Rust**: Latest stable toolchain (`rustup default stable`)
- **Node.js**: v20 or higher
- **pnpm**: v9 or higher

Ensure `~/.cargo/bin` is in your `PATH`:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Installation

```bash
# Clone the repository
git clone https://github.com/minkyet/szz.git
cd szz

# Install frontend dependencies
pnpm install
```

### Running in dev

```bash
# Run web frontend in development mode
pnpm dev

# Run Tauri desktop launcher in development mode
pnpm tauri dev

# Run szz-cli with a dry-run test manifest
cargo run -p szz-cli -- run fixtures/sample-manifest.json --dry-run
```
