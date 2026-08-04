#!/bin/sh
set -eu

binary=$1
shift

if [ "$(basename "$binary")" = "interview-copilot" ]; then
  : "${APPLE_SIGNING_IDENTITY:?Set APPLE_SIGNING_IDENTITY to a valid Apple Development identity}"
  binary_dir=$(CDPATH= cd -- "$(dirname "$binary")" && pwd)
  script_dir=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
  app="$binary_dir/Interview Copilot.app"
  contents="$app/Contents"
  executable="$contents/MacOS/interview-copilot"

  mkdir -p "$contents/MacOS" "$contents/Resources/fixtures/profile-sources"
  cp "$script_dir/../dev/Info.plist" "$contents/Info.plist"
  cp "$binary" "$executable"
  cp "$script_dir/../fixtures/profile-sources/"* "$contents/Resources/fixtures/profile-sources/"
  if [ -d "$contents/_CodeSignature" ]; then
    rm -rf "$contents/_CodeSignature"
  fi

  codesign --force --sign "$APPLE_SIGNING_IDENTITY" --identifier com.interviewcopilot.desktop "$executable"
  codesign --force --sign "$APPLE_SIGNING_IDENTITY" "$app"
  codesign --verify --deep --strict "$app"
  exec "$executable" "$@"
fi

exec "$binary" "$@"
