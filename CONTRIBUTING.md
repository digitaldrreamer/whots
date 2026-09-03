# Contributing to Whots

Thank you for your interest in contributing to Whots! This project thrives on contributions from the community, whether you're reporting bugs, improving documentation, designing new cards, or enhancing the AI.

---

## Code of Conduct

All contributors and participants agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Please report any unacceptable behavior according to the procedures outlined there.

---

## How Can I Contribute?

### 1. Reporting Bugs
- Search existing [GitHub Issues](https://github.com/digitaldrreamer/whots/issues) first to see if the issue has already been reported.
- If not, create a new issue detailing:
  - A clear and descriptive title.
  - Steps to reproduce the problem.
  - Expected vs. actual behavior.
  - Browser/platform and version information.
  - Relevant console or server logs if available.

### 2. Suggesting Enhancements
- Open an issue describing the feature or enhancement, why it would be valuable, and how it might be implemented.

### 3. Submitting Pull Requests
1. **Fork the repository** and create your branch from `main`:
   ```bash
   git checkout -b feature/my-new-feature
   ```
2. **Follow Coding Standards**:
   - For Rust backend:
     - Run `cargo fmt` to ensure standard formatting.
     - Run `cargo clippy -- -D warnings` and fix any lint warnings.
     - Run `cargo test` and ensure all tests pass.
   - For Svelte frontend:
     - Run `npm run lint` and `npm run check`.
     - Format code with Prettier (`npm run format`).
3. **Commit your changes**:
   - Write clear, concise commit messages following the [Conventional Commits](https://www.conventionalcommits.org/) convention (e.g. `feat: ...`, `fix: ...`, `docs: ...`).
4. **Push to your fork** and submit a Pull Request targeting `main`.
5. Ensure CI tests pass on your Pull Request.

---

## Local Development Workflow

Refer to the main [README.md](README.md) for full instructions on spinning up the PostgreSQL and Redis dependencies using Docker Compose, running the Rust server, and starting the SvelteKit frontend.
