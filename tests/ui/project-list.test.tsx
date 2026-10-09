import { screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, projectTiles, renderApp, selectTask, taskHash, toggleTask } from "./helpers";

test("open projects show with their open task count", async () => {
  await addProject({ name: "Alpha", content: "- [ ] one\n- [ ] two\n- [x] done\n" });
  await addProject({ name: "Beta", content: "- [ ] only\n" });
  renderApp();

  expect(await screen.findByRole("button", { name: /^Alpha/ })).toHaveTextContent("2 open");
  expect(screen.getByRole("button", { name: /^Beta/ })).toHaveTextContent("1 open");
});

test("tabs show only their own projects", async () => {
  await addProject({ name: "Work" });
  await addProject({ name: "Maths", type: "learning" });
  const { user } = renderApp();

  await screen.findByRole("button", { name: /^Work/ });
  expect(screen.queryByRole("button", { name: /^Maths/ })).not.toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Learning" }));
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

  await projectTiles().findByRole("button", { name: /^Zulu/ });
  const names = projectTiles()
    .getAllByRole("button")
    .map((b) => b.textContent)
    .filter((t) => !["Projects", "Learning"].includes(t ?? ""));
  expect(names).toEqual([
    expect.stringMatching(/^Zulu2 next of 2 open/),
    expect.stringMatching(/^Bravo/),
    expect.stringMatching(/^Charlie/),
  ]);
});

test("archived projects are collapsed until the section is opened", async () => {
  await addProject({ name: "Live" });
  await addProject({ name: "Shipped", status: "archived", sub_status: "done" });
  const { user } = renderApp();

  const header = await screen.findByRole("button", { name: "Archived (1)" });
  expect(screen.queryByText("Shipped")).not.toBeInTheDocument();

  await user.click(header);
  expect(screen.getByRole("button", { name: /^Shipped/ })).toHaveTextContent("· done");
});

test("no archived project, no archived section", async () => {
  await addProject({ name: "Live" });
  renderApp();
  await projectTiles().findByRole("button", { name: /^Live/ });

  expect(screen.queryByRole("button", { name: /^Archived/ })).not.toBeInTheDocument();
});

test("an empty list says so", async () => {
  renderApp();
  expect(await screen.findByText("No projects yet.")).toBeInTheDocument();
});

test("a tile shows its last 8 weeks of activity", async () => {
  const project = await addProject({ name: "Busy", content: "- [ ] a\n- [ ] b\n" });
  await toggleTask(project.id, await taskHash(project.id, "a"));
  await toggleTask(project.id, await taskHash(project.id, "b"));
  renderApp();

  const tile = await projectTiles().findByRole("button", { name: /^Busy/ });
  const row = tile.querySelector("[data-weeks]") as HTMLElement;
  await waitFor(() => expect(row.dataset.weeks).toBe("0,0,0,0,0,0,0,2"));
});

test("the open project's tile is ringed", async () => {
  await addProject({ name: "Work" });
  const { user } = renderApp();
  const tile = await projectTiles().findByRole("button", { name: /^Work/ });
  expect(tile).not.toHaveClass("ring-next");

  await user.click(tile);

  expect(tile).toHaveClass("ring-next");
});
