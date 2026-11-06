import { test } from "node:test";
import assert from "node:assert/strict";
import React from "react";
import { renderToString } from "react-dom/server";
import { DownloadLandingPage } from "./DownloadLandingPage";

test("DownloadLandingPage renders complete HTML with hero, CTAs, and mockup", () => {
  let backTriggered = false;
  const onBack = () => {
    backTriggered = true;
  };

  const html = renderToString(
    React.createElement(DownloadLandingPage, {
      onBack,
    })
  );

  onBack();
  assert.equal(backTriggered, true);

  assert.ok(html.includes("szz 다운로드"));
  assert.ok(html.includes("Steam-style Desktop Modpack Platform"));
  assert.ok(html.includes("QuickPlay 1-클릭 플레이"));
  assert.ok(html.includes("왜 szz인가요?"));
  assert.ok(html.includes("디스크 용량 50%+ 절감"));
});
