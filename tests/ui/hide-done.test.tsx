import { cleanup, render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "@/App";
import { addProject, openProject, renderApp, taskPane } from "./helpers";

test("hide done is saved per project and survives a restart", async () => {
  await addProject({ name: "Alpha", content: "## M\n- [x] shipped\n- [ ] open one\n" });
  await addProject({ name: "Beta", content: "- [x] beta done\n- [ ] beta open\n" });
  const { user } = renderApp();
  await openProject(user, "Alpha");
  expect(taskPane().getByText("shipped")).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Hide done" }));

  expect(taskPane().queryByText("shipped")).not.toBeInTheDocument();
  expect(taskPane().getByText("open one")).toBeInTheDocument();
  // The milestone keeps its full progress.
  expect(taskPane().getByText("1 of 2 done")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "1 done hidden" })).toHaveAttribute("aria-pressed", "true");

  await user.click(screen.getByRole("button", { name: "Close project" }));
  await openProject(user, "Beta");
  expect(taskPane().getByText("beta done")).toBeInTheDocument();

  cleanup();
  render(<App />);
  await openProject(user, "Alpha");
  expect(taskPane().queryByText("shipped")).not.toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "1 done hidden" }));
  expect(taskPane().getByText("shipped")).toBeInTheDocument();
});

test("when every task is done and hidden, the pane says so", async () => {
  await addProject({ name: "Done", content: "## M\n- [x] a\n- [x] b\n" });
  const { user } = renderApp();
  await openProject(user, "Done");

  await user.click(screen.getByRole("button", { name: "Hide done" }));

  expect(taskPane().getByText(/All tasks are done/)).toBeInTheDocument();
  expect(taskPane().queryByText("M")).not.toBeInTheDocument();
  await user.click(screen.getByRole("button", { name: "Show done" }));
  expect(taskPane().getByText("a")).toBeInTheDocument();
});

test("no done task, no Hide done button", async () => {
  await addProject({ name: "Fresh", content: "- [ ] a\n" });
  const { user } = renderApp();
  await openProject(user, "Fresh");

  expect(screen.queryByRole("button", { name: "Hide done" })).not.toBeInTheDocument();
});
