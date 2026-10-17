import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import App from "@/App";
import {
  addProject,
  addTask,
  delegate,
  inboxTasks,
  linkTools,
  openProject,
  renderApp,
  selectTask,
  taskHash,
  taskPane,
  toggleTask,
} from "./helpers";

/** A filter pill of the task pane's header (a next task's own "Next" pill isn't one). */
const filter = (name: string) =>
  screen.getAllByRole("button", { name }).find((b) => b.hasAttribute("aria-pressed"))!;

test("Next is saved per project and survives a restart", async () => {
  const alpha = await addProject({ name: "Alpha", content: "- [ ] first\n- [ ] second\n" });
  await selectTask(alpha.id, await taskHash(alpha.id, "first"));
  const beta = await addProject({ name: "Beta", content: "- [ ] beta one\n- [ ] beta two\n" });
  await selectTask(beta.id, await taskHash(beta.id, "beta one"));
  const { user } = renderApp();
  await openProject(user, "Alpha");

  await user.click(filter("Next"));
  expect(taskPane().queryByText("second")).not.toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Close project" }));
  await openProject(user, "Beta");
  expect(taskPane().getByText("beta two")).toBeInTheDocument();

  cleanup();
  render(<App />);
  await openProject(user, "Alpha");
  expect(filter("Next")).toHaveAttribute("aria-pressed", "true");
  expect(taskPane().getByText("first")).toBeInTheDocument();
  expect(taskPane().queryByText("second")).not.toBeInTheDocument();

  await user.click(filter("Next"));
  cleanup();
  render(<App />);
  await openProject(user, "Alpha");
  expect(taskPane().getByText("second")).toBeInTheDocument();
});

test("Delegated is saved per project", async () => {
  const p = await addProject({ name: "P", content: "- [ ] plain\n" });
  await addTask(p.id, "mail");
  await linkTools(p.id, "writer");
  await delegate(p.id, "mail", "writer");
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(filter("Delegated"));
  cleanup();
  render(<App />);
  await openProject(user, "P");

  expect(filter("Delegated")).toHaveAttribute("aria-pressed", "true");
  expect(taskPane().queryByText("plain")).not.toBeInTheDocument();
});

test("Inbox isn't saved", async () => {
  const p = await addProject({ name: "P", content: "- [ ] alpha\n- [ ] beta\n" });
  await inboxTasks(p.id, "beta");
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(filter("Inbox"));
  cleanup();
  render(<App />);
  await openProject(user, "P");

  expect(filter("Inbox")).toHaveAttribute("aria-pressed", "false");
  expect(taskPane().getByText("alpha")).toBeInTheDocument();
});

test("a saved filter with nothing left to show turns off", async () => {
  const p = await addProject({ name: "P", content: "- [ ] first\n- [ ] second\n" });
  const first = await taskHash(p.id, "first");
  await selectTask(p.id, first);
  const { user } = renderApp();
  await openProject(user, "P");
  await user.click(filter("Next"));
  cleanup();

  // Done outside the app: no next task left.
  await toggleTask(p.id, first);
  render(<App />);
  await openProject(user, "P");

  await waitFor(() => expect(taskPane().getByText("second")).toBeInTheDocument());
  expect(screen.queryByRole("button", { name: "Next" })).not.toBeInTheDocument();
  expect(localStorage.getItem(`pov.only.${p.id}`)).toBe(null);
});
