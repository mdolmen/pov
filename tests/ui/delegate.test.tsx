import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { expect, test } from "vitest";
import {
  addProject,
  addTask,
  checkbox,
  delegate,
  finishTask,
  linkTools,
  openProject,
  renderApp,
  taskHash,
  taskPane,
  taskRow,
} from "./helpers";

/** The band on a task's row: "next", "inbox", "delegated" or null. */
const band = (text: string) =>
  taskRow(text).style.boxShadow.match(/--pov-(next|inbox|delegated)/)?.[1] ?? null;

const delegateButton = (text: string) =>
  within(taskRow(text)).queryByRole("button", { name: "Delegate" });

interface Options {
  tools?: string[];
  delegateError?: string;
  /** Tasks delegated to the first tool before the app opens. */
  delegated?: string[];
}

/** Opens project "P": a plain task, a done one, and "mail" and "other" with metadata, in the inbox. */
async function setup({ tools = ["writer"], delegateError, delegated = [] }: Options = {}) {
  const p = await addProject({ name: "P", content: "- [ ] plain\n- [x] finished\n" });
  await addTask(p.id, "mail", { source: { mail: "m.eml" } });
  await addTask(p.id, "other", { source: { mail: "o.eml" } });
  if (tools.length > 0) linkTools(p.id, ...tools);
  for (const text of delegated) await delegate(p.id, text, tools[0]);
  const { user, calls } = renderApp({ delegateError });
  await openProject(user, "P");
  return { user, calls, project: p };
}

test("no Delegate in a project without tools", async () => {
  await setup({ tools: [] });

  expect(delegateButton("mail")).not.toBeInTheDocument();
});

test("Delegate shows on open tasks, with or without metadata; with one tool it asks for a note", async () => {
  const { user } = await setup();
  expect(delegateButton("plain")).toBeInTheDocument();
  expect(delegateButton("finished")).not.toBeInTheDocument();

  await user.click(delegateButton("mail")!);

  const dialog = await screen.findByRole("dialog", { name: "Delegate to writer" });
  expect(within(dialog).getByRole("textbox", { name: "Note" })).toBeInTheDocument();
});

test("with several tools, Delegate opens a menu of them", async () => {
  const { user } = await setup({ tools: ["writer", "coder"] });

  await user.click(delegateButton("mail")!);
  expect(await screen.findByRole("menuitem", { name: "writer" })).toBeInTheDocument();
  await user.click(screen.getByRole("menuitem", { name: "coder" }));

  expect(await screen.findByRole("dialog", { name: "Delegate to coder" })).toBeInTheDocument();
});

test("a delegated task has the band and its tool's pill, and no actions", async () => {
  const { user, calls, project } = await setup();
  const hash = await taskHash(project.id, "mail");

  await user.click(delegateButton("mail")!);
  const dialog = await screen.findByRole("dialog", { name: "Delegate to writer" });
  await user.type(within(dialog).getByRole("textbox", { name: "Note" }), "Draft a reply");
  await user.click(within(dialog).getByRole("button", { name: "Delegate" }));

  expect(calls).toContainEqual([
    "delegate",
    { projectId: project.id, taskHash: hash, tool: "writer", note: "Draft a reply" },
  ]);
  await waitFor(() => expect(band("mail")).toBe("delegated"));
  const row = within(taskRow("mail"));
  expect(row.getByText("writer")).toBeInTheDocument();
  for (const name of ["Mark next", "Keep", "Delete task", "Delegate"]) {
    expect(row.queryByRole("button", { name })).not.toBeInTheDocument();
  }
  expect(checkbox("mail")).toBeDisabled();

  await user.click(screen.getByRole("button", { name: "Close project" }));
  await openProject(user, "P");
  expect(band("mail")).toBe("delegated");
});

test("a tool that fails to start shows why; the task stays delegated", async () => {
  const { user, calls } = await setup({ delegateError: "could not start writer: not found" });

  await user.click(delegateButton("mail")!);
  const dialog = await screen.findByRole("dialog", { name: "Delegate to writer" });
  await user.click(within(dialog).getByRole("button", { name: "Delegate" }));

  expect(await screen.findByRole("alert")).toHaveTextContent("could not start writer: not found");
  expect(calls.find(([cmd]) => cmd === "delegate")?.[1]).toMatchObject({ note: null });
  expect(band("mail")).toBe("delegated");
});

test("Delegated shows only delegated tasks; a finished one comes back to the inbox", async () => {
  const { user, project } = await setup({ delegated: ["mail"] });
  const filter = screen.getByRole("button", { name: "Delegated", pressed: false });

  await user.click(filter);

  expect(taskPane().getByText("mail")).toBeInTheDocument();
  for (const text of ["plain", "finished", "other"]) {
    expect(taskPane().queryByText(text)).not.toBeInTheDocument();
  }

  await finishTask(project.id, "mail");
  fireEvent.focus(window);

  await waitFor(() => expect(band("mail")).toBe("inbox"));
  expect(screen.queryByRole("button", { name: "Delegated" })).not.toBeInTheDocument();
  expect(taskPane().getByText("plain")).toBeInTheDocument();
});

test("the info icon shows the note and opens the links", async () => {
  const p = await addProject({ name: "P", content: "" });
  await addTask(p.id, "mail", {
    note: "Reply by Friday",
    links: [
      { label: "Draft", target: "https://example.com/draft" },
      { label: "Mail", target: "/tmp/m.eml" },
    ],
  });
  await addTask(p.id, "bare", {});
  const { user, calls } = renderApp();
  await openProject(user, "P");

  await user.click(within(taskRow("mail")).getByRole("button", { name: "Task details" }));
  const popover = await screen.findByRole("dialog", { name: "Details" });
  expect(popover).toHaveTextContent("Reply by Friday");
  await user.click(within(popover).getByRole("button", { name: "Draft" }));
  await user.click(within(popover).getByRole("button", { name: "Mail" }));

  expect(calls).toContainEqual([
    "plugin:opener|open_url",
    expect.objectContaining({ url: "https://example.com/draft" }),
  ]);
  expect(calls).toContainEqual([
    "plugin:opener|open_path",
    expect.objectContaining({ path: "/tmp/m.eml" }),
  ]);
  await user.keyboard("{Escape}");
  await user.click(within(taskRow("bare")).getByRole("button", { name: "Task details" }));
  expect(await screen.findByText("No note or links yet.")).toBeInTheDocument();
});
