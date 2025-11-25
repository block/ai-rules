# Contributing to AI Rules
## Development

1. Clone project:
   ```bash
   git clone https://github.com/block/ai-rules.git
   cd ai-rules
   ```
2. Build the project:
   ```bash
   cargo build
   ```
3. Run tests:
   ```bash
   cargo test
   ```
4. Format:
   ```bash
   cargo fmt --check # checking format

   cargo fmt # applying formatting automatically
   ```
5. Lint (Clippy):
   ```bash
   ./scripts/clippy-check.sh # run clippy

   ./scripts/clippy-fix.sh # run clippy autofix
   ```
6. Run CLI locally:
   ```bash
   cargo build

   ./target/debug/ai-rules --help
   ```
## CI

[Build Pipeline](https://github.com/block/ai-rules/actions/workflows/ci.yml)

## Creating a fork

To fork the repository:

1. Go to https://github.com/block/ai-rules and click “Fork” (top-right corner).
2. This creates https://github.com/<your-username>/ai-rules under your GitHub account.
3. Clone your fork (not the main repo):

```
git clone https://github.com/<your-username>/ai-rules.git
cd ai-rules
```

4. Add the main repository as upstream:

```
git remote add upstream https://github.com/block/ai-rules.git
```

5. Create a branch in your fork for your changes:

```
git checkout -b my-feature-branch
```

6. Sync your fork with the main repo:

```
git fetch upstream

# Merge them into your local branch (e.g., 'main' or 'my-feature-branch')
git checkout main
git merge upstream/main
```

7. Push to your fork. Because you’re the owner of the fork, you have permission to push here.

```
git push origin my-feature-branch
```

8. Open a Pull Request from your branch on your fork to block/ai-rules’s main branch.

## Keeping Your Fork Up-to-Date

To ensure a smooth integration of your contributions, it's important that your fork is kept up-to-date with the main repository. This helps avoid conflicts and allows us to merge your pull requests more quickly. Here’s how you can sync your fork:

### Syncing Your Fork with the Main Repository

1. **Add the Main Repository as a Remote** (Skip if you have already set this up):

   ```bash
   git remote add upstream https://github.com/block/ai-rules.git
   ```

2. **Fetch the Latest Changes from the Main Repository**:

   ```bash
   git fetch upstream
   ```

3. **Checkout Your Development Branch**:

   ```bash
   git checkout your-branch-name
   ```

4. **Merge Changes from the Main Branch into Your Branch**:

   ```bash
   git merge upstream/main
   ```

   Resolve any conflicts that arise and commit the changes.

5. **Push the Merged Changes to Your Fork**:

   ```bash
   git push origin your-branch-name
   ```

This process will help you keep your branch aligned with the ongoing changes in the main repository, minimizing integration issues when it comes time to merge!

### Before Submitting a Pull Request

Before you submit a pull request, please ensure your fork is synchronized as described above. This check ensures your changes are compatible with the latest in the main repository and streamlines the review process.

If you encounter any issues during this process or have any questions, please reach out by opening an issue, and we'll be happy to help.