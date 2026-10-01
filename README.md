# ⚡ The Rishikesh Programming Language (`.rk`)

<div align="center">

```
  ____  _     _     _ _              _     
 |  _ \(_)___| |__ (_) | _____  ___ | |__  
 | |_) | / __| '_ \| | |/ / _ \/ __|| '_ \ 
 |  _ <| \__ \ | | | |   <  __/\__ \| | | |
 |_| \_\_|___/_| |_|_|_|\_\___||___/|_| |_|
```

[![CI](https://img.shields.io/badge/CI-Passing-10b981?style=for-the-badge&logo=githubactions&logoColor=white)](https://github.com/kingroyale537/rishikesh/actions)
[![Version](https://img.shields.io/badge/version-0.1.0-38bdf8?style=for-the-badge)](https://github.com/kingroyale537/rishikesh/releases)
[![Open VSX](https://img.shields.io/badge/Open%20VSX-Verified%20Publisher-10b981?style=for-the-badge&logo=visualstudiocode&logoColor=white)](https://open-vsx.org/extension/kingroyale537/vscode-rishikesh)
[![Platforms](https://img.shields.io/badge/Platforms-macOS%20|%20Linux%20|%20Windows-f59e0b?style=for-the-badge)](https://github.com/kingroyale537/rishikesh)
[![License](https://img.shields.io/badge/license-MIT%20/%20Apache--2.0-ec4899?style=for-the-badge)](LICENSE)

**The Universal, High-Performance Programming Language for AI, Systems, and Cloud.**  
Designed to unify **Python's elegance**, **Rust's zero-cost bare-metal performance**, and **Erlang's actor concurrency**.

[Website](https://rishikesh.lang) • [Interactive Playground](https://rishikesh.lang#playground) • [Documentation](docs/) • [Report Bug](https://github.com/kingroyale537/rishikesh/issues)

</div>

---

## ⚡ 1-Line Universal Installation

### macOS, Linux & WSL
```bash
curl -fsSL https://rishikesh.lang/install.sh | sh
```

### macOS via Homebrew
```bash
brew tap rishikesh/tap
brew install rishi
```

### Windows (PowerShell)
```powershell
iwr -useb https://rishikesh.lang/install.ps1 | iex
```

### From Source (Requires Rust 1.75+)
```bash
git clone https://github.com/kingroyale537/rishikesh.git
cd rishikesh
cargo install --path crates/rishi_cli
```

---

## 🏛️ Why Rishikesh Beats the Alternatives

| Feature | Rishikesh (`.rk`) | Python | Rust | Go |
| :--- | :--- | :--- | :--- | :--- |
| **Package Reach** | **10,000,000+ (Omniverse)** | 550,000 (PyPI) | 170,000 (Crates) | 1,200,000 (Go Modules) |
| **Integer Math** | **Arbitrary-Precision (No Overflow)** | Arbitrary-Precision | Fixed $i64$ (Overflows) | Fixed $i64$ (Overflows) |
| **Memory Control** | **Zero-Copy Buffers + Cycle GC** | Ref-count + Cycle GC | Borrow Checker | Tracing GC |
| **Concurrency** | **Fault-Tolerant Actors & Mailboxes** | GIL-limited / asyncio | Threads / Tokio | Goroutines |
| **Built-in Tooling** | **fmt, lint, test, profile, replay** | Fragmented | cargo, clippy | go fmt, go test |

---

## 🚀 Key Architectural Innovations

### 1. The Omniverse: 10,000,000+ Federated Packages
Never rewrite libraries again. Rishikesh transparently federates and executes packages across **PyPI, npm, and crates.io** in a single process:
```bash
rishi add py:torch@2.1.0        # From Python / PyPI
rishi add npm:three@0.160.0     # From JavaScript / npm
rishi add cargo:tokio@1.35.0    # From Rust / crates.io
```

### 2. Native AI Autograd & SIMD Vectorization
Run tensor operations and neural network backpropagation with zero external dependencies:
```rk
let v1 = [1.0, 2.0, 3.0, 4.0]
let v2 = [5.0, 6.0, 7.0, 8.0]
let dot = simd_dot(v1, v2)
println("Hardware SIMD Dot Product: {dot}")
```

### 3. Arbitrary-Precision BigInt by Default
Exact precision math without $2^{63}-1$ integer overflow:
```rk
let astronomical = big_pow(2, 1000)      # Computes 302 digits seamlessly
let combinatorics = big_factorial(100)   # Computes 158 digits
```

### 4. Time-Traveling Deterministic Replay Debugger
Record every variable mutation and jump backward in execution time:
```bash
rishi run script.rk --record=trace.json
rishi replay trace.json
```

### 5. Built-in Runtime Profiler & Flamegraph
```bash
rishi profile script.rk
```

---

## 🛠️ Complete CLI Command Suite

```bash
rishi run <file.rk>       # Execute script
rishi repl                # Launch interactive shell
rishi check <file.rk>     # Fast syntax validation
rishi fmt .               # Auto-format all project files
rishi lint <file.rk>      # Best-practice static linter
rishi test                # Run project test suites
rishi profile <file.rk>   # CPU flamegraph and memory breakdown
rishi add <pkg>           # Resolve and install package
rishi replay <trace.json> # Deterministic execution replayer
```

---

## 🎨 VS Code Editor Extension

The official Rishikesh VS Code extension provides:
* **Rich Syntax Highlighting:** Keywords, types, pipelines (`|>`), decorators (`@kernel`, `@server`, `@ui`).
* **Snippets & Autocomplete:** Pre-built templates for structs, actors, autograd, and pipelines.
* **Auto-Formatting on Save:** Powered natively by `rishi fmt`.

Install locally by opening `editors/vscode-rishikesh` in VS Code or running:
```bash
code --install-extension editors/vscode-rishikesh
```

---

## 📜 License

Rishikesh is dual-licensed under the **MIT License** and the **Apache 2.0 License**.

---

<div align="center">
<b>Designed with ❤️ by Rishikesh Rai. Built for the next century of computing.</b>
</div>
