import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";
import { afterEach, beforeEach } from "vitest";
import { resetData } from "./helpers";

// jsdom has no layout, so no scrolling; the task pane scrolls to the picked Next up entry.
Element.prototype.scrollIntoView = () => {};

beforeEach(async () => {
  await resetData();
});

afterEach(() => {
  cleanup();
  clearMocks();
  localStorage.clear();
});
