# Release Process

## Steps

1. **Bump the version in `Cargo.toml` and merge to `main`**

2. **Select the commit SHA from `main` for release**

3. **Verify the commit has the target version in `Cargo.toml`**

4. **Run the release script**
   ```bash
   ./scripts/trigger-release.sh <commit-sha-on-main>
   ```

5. **Monitor the release**
   - Workflow: https://github.com/block/ai-rules/actions/workflows/release.yml
   - Release: https://github.com/block/ai-rules/releases