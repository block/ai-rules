# Release Process

## Steps

1. **Select commit SHA from `main` for release**

2. **Verify the commit has the target version in `Cargo.toml`**

3. **Run the release script**
   ```bash
   ./scripts/trigger-release.sh <commit-sha-on-main>
   ```

4. **Monitor the release**
   - Workflow: https://github.com/block/ai-rules/actions/workflows/release.yml
   - Release: https://github.com/block/ai-rules/releases