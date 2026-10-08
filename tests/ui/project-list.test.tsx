import { screen, within } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, renderApp, selectTask, taskHash } from "./helpers";

test("open projects show with their task count", async () => {
  await addProject({ name: "Alpha", content: "- [ ] one\n- [ ] two\n- [x] done\n" });
  await addProject({ name: "Beta", content: "- [ ] only\n" });
  renderApp();

  expect(await screen.findByRole("button", { name: /^Alpha/ })).toHaveTextContent("2 tasks");
  expect(screen.getByRole("button", { name: /^Beta/ })).toHaveTextContent("1 task");
});

test("tabs show only their own projects", async () => {
  await addProject({ name: "Work" });
  await addProject({ name: "Maths", type: "learning" });
  const { user } = renderApp();

  await screen.findByRole("button", { name: /^Work/ });
  expect(screen.queryByRole("button", { name: /^Maths/ })).not.toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "learning" }));
  await screen.findByRole("button", { name: /^Maths/ });
  expect(screen.queryByRole("button", { name: /^Work/ })).not.toBeInTheDocument();
});

test("projects are ordered by next task count, then by name", async () => {
  await addProject({ name: "Charlie" });
  await addProject({ name: "Bravo" });
  const busy = await addProject({ name: "Zulu", content: "- [ ] a\n- [ ] b\n" });
  await selectTask(busy.id, await taskHash(busy.id, "a"));
  await selectTask(busy.id, await taskHash(busy.id, "b"));
  renderApp();

  await screen.findByRole("button", { name: /^Zulu/ });
  const open = screen.getByText("Open").parentElement as HTMLElement;
  const names = within(open).getAllByRole("button").map((b) => b.textContent);
  expect(names).toEqual([
    expect.stringMatching(/^Zulu2/),
    expect.stringMatching(/^Bravo/),
    expect.stringMatching(/^Charlie/),
  ]);
});

test("archived projects are collapsed until the section is opened", async () => {
  await addProject({ name: "Live" });
  await addProject({ name: "Shipped", status: "archived", sub_status: "done" });
  const { user } = renderApp();
  await user.click(await screen.findByRole("button", { name: "Menu" }));
  await user.click(screen.getAllByRole("button", { name: "Archived" })[0]);
  await user.click(screen.getByRole("button", { name: "Menu" }));

  const header = await screen.findByRole("button", { name: "Archived (1)" });
  expect(screen.queryByText("Shipped")).not.toBeInTheDocument();

  await user.click(header);
  expect(screen.getByRole("button", { name: /^Shipped/ })).toHaveTextContent("· done");
});

test("an empty list says so", async () => {
  renderApp();
  expect(await screen.findByText("No projects yet.")).toBeInTheDocument();
});
