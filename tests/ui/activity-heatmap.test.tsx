import { cleanup, render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "@/App";
import { addProject, renderApp, taskHash, today, toggleTask } from "./helpers";

test("a day with commits shows its count, per tab", async () => {
  const project = await addProject({ name: "P", content: "- [ ] a\n- [ ] b\n" });
  await toggleTask(project.id, await taskHash(project.id, "a"));
  await toggleTask(project.id, await taskHash(project.id, "b"));
  const { user } = renderApp();

  expect(await screen.findByText(`2 activities on ${today()}`)).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "learning" }));
  expect(await screen.findByText(`0 activities on ${today()}`)).toBeInTheDocument();
});

test("hide and show are remembered per tab", async () => {
  const { user } = renderApp();
  await screen.findByRole("img", { name: "Heatmap" });

  await user.click(screen.getByRole("button", { name: "Hide" }));
  expect(screen.queryByRole("img", { name: "Heatmap" })).not.toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "learning" }));
  expect(await screen.findByRole("img", { name: "Heatmap" })).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "projects" }));
  expect(screen.getByRole("button", { name: "Show activity" })).toBeInTheDocument();

  // Still hidden after a restart.
  cleanup();
  render(<App />);
  expect(await screen.findByRole("button", { name: "Show activity" })).toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Show activity" }));
  expect(await screen.findByRole("img", { name: "Heatmap" })).toBeInTheDocument();
});
