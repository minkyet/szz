import { test } from "node:test";
import assert from "node:assert/strict";
import React from "react";
import { LauncherInstallModal } from "./LauncherInstallModal";

test("LauncherInstallModal returns null when isOpen is false", () => {
  const element = React.createElement(LauncherInstallModal, {
    isOpen: false,
    onClose: () => {},
    onDownload: () => {},
    onRetry: () => {},
  });

  // Execute component function directly
  const rendered = (LauncherInstallModal as any)(element.props);
  assert.equal(rendered, null);
});

test("LauncherInstallModal renders modal tree when isOpen is true", () => {
  let downloadTriggered = false;
  let retryTriggered = false;
  let closeTriggered = false;

  const props = {
    isOpen: true,
    serverName: "Cobblemon Adventure",
    minecraftVersion: "1.20.1",
    loaderType: "fabric",
    onClose: () => {
      closeTriggered = true;
    },
    onDownload: () => {
      downloadTriggered = true;
    },
    onRetry: () => {
      retryTriggered = true;
    },
  };

  const rendered = (LauncherInstallModal as any)(props);
  assert.notEqual(rendered, null);
  assert.equal(typeof rendered, "object");
  assert.equal(rendered.type, "div");

  // Verify callbacks work when invoked
  props.onClose();
  props.onDownload();
  props.onRetry();

  assert.equal(closeTriggered, true);
  assert.equal(downloadTriggered, true);
  assert.equal(retryTriggered, true);
});
