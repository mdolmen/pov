import { cleanup, render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "@/App";
import { addProject, renderApp, taskHash, today, toggleTask } from "./helpers";

test("the strip shows the last 30 days of the tab", async () => {
  const project = await addProject({ name: "P", content: "- [ ] a\n- [ ] b\n" });
  await toggleTask(project.id, await taskHash(project.id, "a"));
  await toggleTask(project.id, await taskHash(project.id, "b"));
  const { user } = renderApp();

  const strip = await screen.findByRole("button", { name: "Show activity" });
  expect(await screen.findByTitle(`2 on ${today()}`)).toBeInTheDocument();
  expect(strip.querySelectorAll("i")).toHaveLength(30);

  await user.click(screen.getByRole("button", { name: "Learning" }));
  expect(await screen.findByTitle(`0 on ${today()}`)).toBeInTheDocument();
});

test("clicking the strip shows the full heatmap, per tab", async () => {
  const project = await addProject({ name: "P", content: "- [ ] a\n" });
  await toggleTask(project.id, await taskHash(project.id, "a"));
  const { user } = renderApp();

  await user.click(await screen.findByRole("button", { name: "Show activity" }));
  expect(await screen.findByText(`1 activity on ${today()}`)).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Learning" }));
  expect(screen.getByRole("button", { name: "Show activity" })).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Projects" }));
  expect(screen.getByRole("img", { name: "Heatmap" })).toBeInTheDocument();

  // Still open after a restart; Hide goes back to the strip.
  cleanup();
  render(<App />);
  expect(await screen.findByRole("img", { name: "Heatmap" })).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Hide" }));
  expect(screen.getByRole("button", { name: "Show activity" })).toBeInTheDocument();
});
