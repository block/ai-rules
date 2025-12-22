#!/bin/bash
set -e

cargo build --release

mkdir -p /tmp/ai-rules-test/ai-rules
cd /tmp/ai-rules-test

cat > ai-rules/regular-rule.md << 'EOF'
---
description: Regular rule
alwaysApply: true
---
# Regular Rule
This is a regular file.
EOF

cat > /tmp/shared-rule.md << 'EOF'
---
description: Shared rule
alwaysApply: true
---
# Shared Rule
This is a symlinked file.
EOF

ln -s /tmp/shared-rule.md ai-rules/shared-rule.md

ls -la ai-rules/

echo "=== Test 1: Default behavior - symlinks should be included ==="
~/Development/sourcery/ai-rules/target/release/ai-rules generate
cat ai-rules/.generated-ai-rules/ai-rules-generated-index.md

echo -e "\n=== Test 2: With --no-follow-symlinks - symlinks should be excluded ==="
~/Development/sourcery/ai-rules/target/release/ai-rules clean
~/Development/sourcery/ai-rules/target/release/ai-rules generate --no-follow-symlinks
cat ai-rules/.generated-ai-rules/ai-rules-generated-index.md

cd ~
rm -rf /tmp/ai-rules-test /tmp/shared-rule.md
