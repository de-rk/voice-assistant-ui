#!/bin/bash
# VoiceAssistant uninstall — removes app data and cache from macOS
set -e

APP_SUPPORT="$HOME/Library/Application Support/com.voiceassistant.menubar"
APP_CACHE="$HOME/Library/Caches/com.voiceassistant.menubar"
LEGACY="$HOME/.voice-assistant"

echo "Removing VoiceAssistant data..."
for dir in "$APP_SUPPORT" "$APP_CACHE" "$LEGACY"; do
    if [ -d "$dir" ]; then
        rm -rf "$dir" && echo "  Removed: $dir"
    fi
done
echo "Done. Drag VoiceAssistant.app to Trash to complete uninstall."
