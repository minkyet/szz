import test from "node:test";
import assert from "node:assert/strict";
import { detectClientOS, RELEASE_ASSETS } from "./useOsDetector";

test("detectClientOS detects Windows correctly from UA and platform", () => {
  assert.equal(
    detectClientOS("Mozilla/5.0 (Windows NT 10.0; Win64; x64)", "Win32"),
    "windows"
  );
  assert.equal(
    detectClientOS("Mozilla/5.0 (Windows NT 11.0; Win64; x64)"),
    "windows"
  );
});

test("detectClientOS detects macOS correctly from UA and platform", () => {
  assert.equal(
    detectClientOS(
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
      "MacIntel"
    ),
    "macos"
  );
});

test("detectClientOS detects Linux correctly from UA and platform", () => {
  assert.equal(
    detectClientOS("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36", "Linux x86_64"),
    "linux"
  );
});

test("detectClientOS handles mobile devices (Android/iOS) as unknown", () => {
  assert.equal(
    detectClientOS("Mozilla/5.0 (Linux; Android 13; SM-S901B) AppleWebKit/537.36", "Linux armv8l"),
    "unknown"
  );
  assert.equal(
    detectClientOS("Mozilla/5.0 (iPhone; CPU iPhone OS 16_5 like Mac OS X)", "iPhone"),
    "unknown"
  );
});

test("contains release assets for all supported desktop OSes", () => {
  assert.equal(RELEASE_ASSETS.windows.filename, "szz-setup.exe");
  assert.equal(RELEASE_ASSETS.macos.filename, "szz-universal.dmg");
  assert.equal(RELEASE_ASSETS.linux.filename, "szz-linux-amd64.deb");
});
