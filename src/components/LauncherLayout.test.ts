import { test } from "node:test";
import assert from "node:assert/strict";
import React from "react";
import { LauncherLayout } from "./LauncherLayout";

test("LauncherLayout omits header when isWeb is false", () => {
  const rendered = (LauncherLayout as any)({
    isWeb: false,
    header: React.createElement("div", { id: "test-header" }, "Header"),
    sidebar: React.createElement("aside", { id: "test-sidebar" }, "Sidebar"),
    children: React.createElement("div", { id: "test-content" }, "Content"),
  });

  assert.notEqual(rendered, null);
  assert.equal(rendered.type, "div");

  // Inspect children of <main>
  const mainNode = rendered.props.children[1];
  assert.equal(mainNode.type, "main");
  assert.equal(mainNode.props.children[0], false); // isWeb && header evaluated to false
});

test("LauncherLayout includes header when isWeb is true", () => {
  const headerElement = React.createElement("div", { id: "test-header" }, "Header");
  const rendered = (LauncherLayout as any)({
    isWeb: true,
    header: headerElement,
    sidebar: React.createElement("aside", { id: "test-sidebar" }, "Sidebar"),
    children: React.createElement("div", { id: "test-content" }, "Content"),
  });

  const mainNode = rendered.props.children[1];
  assert.equal(mainNode.type, "main");
  assert.equal(mainNode.props.children[0], headerElement);
});
