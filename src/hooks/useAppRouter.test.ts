import { test } from "node:test";
import assert from "node:assert/strict";
import {
  parsePathnameToRoute,
  parsePathnameToView,
  viewToPathname,
} from "./useAppRouter";

test("parsePathnameToRoute correctly parses view and serverId", () => {
  assert.deepEqual(parsePathnameToRoute("/"), { view: "catalog" });
  assert.deepEqual(parsePathnameToRoute("/servers/cobblemon-1-21-1"), {
    view: "catalog",
    serverId: "cobblemon-1-21-1",
  });
  assert.deepEqual(parsePathnameToRoute("/servers/skyfactory-4/extra/path"), {
    view: "catalog",
    serverId: "skyfactory-4",
  });
  assert.deepEqual(parsePathnameToRoute("/servers/"), { view: "catalog" });
  assert.deepEqual(parsePathnameToRoute("/download"), { view: "download" });
  assert.deepEqual(parsePathnameToRoute("/download/installer"), { view: "download" });
  assert.deepEqual(parsePathnameToRoute("/studio"), { view: "studio" });
  assert.deepEqual(parsePathnameToRoute("/studio/new"), { view: "studio" });
  assert.deepEqual(parsePathnameToRoute(""), { view: "catalog" });
});

test("parsePathnameToView correctly identifies views from path strings (backward compatibility)", () => {
  assert.equal(parsePathnameToView("/"), "catalog");
  assert.equal(parsePathnameToView("/servers/cobblemon"), "catalog");
  assert.equal(parsePathnameToView("/download"), "download");
  assert.equal(parsePathnameToView("/download/windows"), "download");
  assert.equal(parsePathnameToView("/studio"), "studio");
  assert.equal(parsePathnameToView("/studio/new"), "studio");
  assert.equal(parsePathnameToView(""), "catalog");
});

test("viewToPathname produces correct URL paths for router views and server deep links", () => {
  assert.equal(viewToPathname("catalog"), "/");
  assert.equal(viewToPathname("catalog", "cobblemon-1-21-1"), "/servers/cobblemon-1-21-1");
  assert.equal(viewToPathname("catalog", "vanilla paper 26.3"), "/servers/vanilla%20paper%2026.3");
  assert.equal(viewToPathname("download"), "/download");
  assert.equal(viewToPathname("studio"), "/studio");
});
