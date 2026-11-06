import { test } from "node:test";
import assert from "node:assert/strict";
import { SmartDeepLinkDetector } from "./useSmartDeepLink";

test("SmartDeepLinkDetector triggers fallback when window remains focused after timeout", async () => {
  let playCalledWith = "";
  let fallbackCalledWith = "";
  let successCalled = false;

  const playFn = async (id: string) => {
    playCalledWith = id;
  };

  const detector = new SmartDeepLinkDetector(playFn, {
    timeoutMs: 50,
    maxTimeoutMs: 100,
    onFallback: (id) => {
      fallbackCalledWith = id;
    },
    onSuccess: () => {
      successCalled = true;
    },
  });

  const result = await detector.trigger("cobblemon-server");

  assert.equal(playCalledWith, "cobblemon-server");
  assert.equal(result, false); // False indicates fallback was triggered
  assert.equal(fallbackCalledWith, "cobblemon-server");
  assert.equal(successCalled, false);
});

test("SmartDeepLinkDetector marks success when blur occurs and focus does not return", async () => {
  const originalWindow = (globalThis as any).window;
  const originalDocument = (globalThis as any).document;

  const eventListeners: Record<string, Function[]> = {};

  (globalThis as any).window = {
    addEventListener: (event: string, fn: Function) => {
      eventListeners[event] = eventListeners[event] || [];
      eventListeners[event].push(fn);
    },
    removeEventListener: (event: string, fn: Function) => {
      if (eventListeners[event]) {
        eventListeners[event] = eventListeners[event].filter((f) => f !== fn);
      }
    },
  };
  (globalThis as any).document = {
    hidden: false,
    hasFocus: () => false, // External app took focus
    addEventListener: () => {},
    removeEventListener: () => {},
  };

  try {
    let fallbackCalled = false;
    let successCalled = false;

    const playFn = async () => {};
    const detector = new SmartDeepLinkDetector(playFn, {
      timeoutMs: 50,
      maxTimeoutMs: 120,
      onFallback: () => {
        fallbackCalled = true;
      },
      onSuccess: () => {
        successCalled = true;
      },
    });

    const triggerPromise = detector.trigger("test-server");

    // Simulate external app taking focus at 20ms
    setTimeout(() => {
      const blurHandlers = eventListeners["blur"] || [];
      for (const fn of blurHandlers) {
        fn();
      }
    }, 20);

    const result = await triggerPromise;

    assert.equal(result, true);
    assert.equal(fallbackCalled, false);
    assert.equal(successCalled, true);
  } finally {
    (globalThis as any).window = originalWindow;
    (globalThis as any).document = originalDocument;
  }
});

test("SmartDeepLinkDetector marks success when blur occurs followed by prompt closure", async () => {
  const originalWindow = (globalThis as any).window;
  const originalDocument = (globalThis as any).document;

  const eventListeners: Record<string, Function[]> = {};

  (globalThis as any).window = {
    addEventListener: (event: string, fn: Function) => {
      eventListeners[event] = eventListeners[event] || [];
      eventListeners[event].push(fn);
    },
    removeEventListener: (event: string, fn: Function) => {
      if (eventListeners[event]) {
        eventListeners[event] = eventListeners[event].filter((f) => f !== fn);
      }
    },
  };
  (globalThis as any).document = {
    hidden: false,
    hasFocus: () => true, // Browser window regains focus after dialog closes
    addEventListener: () => {},
    removeEventListener: () => {},
  };

  try {
    let fallbackCalled = false;
    let successCalled = false;

    const playFn = async () => {};
    const detector = new SmartDeepLinkDetector(playFn, {
      timeoutMs: 1500, // Long general timeout
      maxTimeoutMs: 3000,
      onFallback: () => {
        fallbackCalled = true;
      },
      onSuccess: () => {
        successCalled = true;
      },
    });

    const triggerPromise = detector.trigger("test-server");

    // 1. Native prompt appears -> blur
    setTimeout(() => {
      for (const fn of eventListeners["blur"] || []) fn();
    }, 20);

    // 2. User clicks Open in native prompt -> dialog closes and focus returns to window
    setTimeout(() => {
      for (const fn of eventListeners["focus"] || []) fn();
    }, 50);

    const result = await triggerPromise;

    // Must resolve as success and NOT trigger fallback!
    assert.equal(result, true);
    assert.equal(fallbackCalled, false);
    assert.equal(successCalled, true);
  } finally {
    (globalThis as any).window = originalWindow;
    (globalThis as any).document = originalDocument;
  }
});

test("SmartDeepLinkDetector can be cancelled cleanly", async () => {
  let fallbackCalled = false;
  const playFn = async () => {};
  const detector = new SmartDeepLinkDetector(playFn, {
    timeoutMs: 50,
    maxTimeoutMs: 100,
    onFallback: () => {
      fallbackCalled = true;
    },
  });

  void detector.trigger("test-server");
  detector.cancel();

  // Wait past the timeout to ensure no lingering timer triggers fallback
  await new Promise((r) => setTimeout(r, 80));
  assert.equal(fallbackCalled, false);
});
