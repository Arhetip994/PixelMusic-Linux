#!/usr/bin/env bash
set -euo pipefail
TOKEN="${1:-${GITHUB_TOKEN:-}}"
if [ -z "$TOKEN" ]; then
    echo "Usage: ./upload_release.sh <GITHUB_PAT_TOKEN>"
    echo "Or set GITHUB_TOKEN environment variable."
    exit 1
fi

REPO="Arhetip994/PixelMusic-Linux"
TAG="v1.0.0-beta"
TITLE="PixelMusic 1.0.0-beta for Linux"
BODY="### PixelMusic 1.0.0-beta Release for Linux

Universal and distribution-specific packages for Linux:
- **AppImage:** \`PixelMusic-1.0.0-beta-x86_64.AppImage\` (Standalone, works on all distributions)
- **Portable:** \`pixelmusic-1.0.0-beta-linux-x86_64.tar.gz\`
- **Debian / Ubuntu / Mint:** \`pixelmusic_1.0.0-beta_amd64.deb\`
- **Arch Linux / CachyOS / Manjaro:** \`pixelmusic-1.0.0-beta-1-x86_64.pkg.tar.zst\`

See \`SHA256SUMS.txt\` for file integrity checks.
"

echo "Creating GitHub Release $TAG..."
RESPONSE=$(curl -fsSL -X POST "https://api.github.com/repos/$REPO/releases" \
    -H "Authorization: token $TOKEN" \
    -H "Accept: application/vnd.github.v3+json" \
    -d "$(python3 -c "import json; print(json.dumps({'tag_name': '$TAG', 'name': '$TITLE', 'body': '''$BODY''', 'draft': False, 'prerelease': True}))")")

UPLOAD_URL=$(python3 -c "import json, sys; print(json.loads(sys.argv[1])['upload_url'].split('{')[0])" "$RESPONSE")
echo "Upload URL: $UPLOAD_URL"

upload_asset() {
    local file="$1"
    local name=$(basename "$file")
    local mime="application/octet-stream"
    echo "Uploading $name..."
    curl -fsSL -X POST "$UPLOAD_URL?name=$name" \
        -H "Authorization: token $TOKEN" \
        -H "Content-Type: $mime" \
        --data-binary @"$file" >/dev/null
    echo "Uploaded $name"
}

for f in releases/*; do
    if [ -f "$f" ]; then
        upload_asset "$f"
    fi
done

echo "Release $TAG published successfully!"
