import { screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import {
  addProject,
  checkbox,
  nextButton,
  nextUp,
  openProject,
  renderApp,
  selectTask,
  taskHash,
} from "./helpers";

/** Marks tasks next through the backend. */
async function markNext(projectId: string, ...texts: string[]) {
  for (const text of texts) await selectTask(projectId, await taskHash(projectId, text));
}

/** The Next up rows, as their text. */
const rows = () => nextUp().getAllByRole("button").map((b) => b.textContent);

test("one entry per project: its first milestone with next tasks", async () => {
  const p = await addProject({
    name: "Book",
    content: "# Book\n## Draft\n- [x] outline\n- [ ] chapter one\n## Review\n- [ ] proofread\n",
  });
  await markNext(p.id, "chapter one", "proofread");
  renderApp();

  await waitFor(() => expect(rows()).toEqual(["Book1 nextDraft1 of 2 done"]));
  expect(screen.getByRole("heading", { name: "Next up 1" })).toBeInTheDocument();
});

test("next tasks before any milestone show as the task, with its subtasks as progress", async () => {
  const p = await addProject({
    name: "Stats",
    content: "- [ ] bootstrap\n  - [x] sample\n  - [ ] plot\n- [ ] read\n## Later\n- [ ] more\n",
  });
  await markNext(p.id, "bootstrap");
  renderApp();

  await waitFor(() => expect(rows()).toEqual(["Statsbootstrap1 of 2 done"]));
});

test("entries are ordered by next count, then by name", async () => {
  const two = "## M\n- [ ] a\n- [ ] b\n";
  const charlie = await addProject({ name: "Charlie", content: two });
  const bravo = await addProject({ name: "Bravo", content: two });
  const zulu = await addProject({ name: "Zulu", content: two });
  await markNext(charlie.id, "a");
  await markNext(bravo.id, "a");
  await markNext(zulu.id, "a", "b");
  renderApp();

  await waitFor(() =>
    expect(rows().map((r) => r?.slice(0, 5))).toEqual(["Zulu2", "Bravo", "Charl"]),
  );
});

test("clicking an entry opens its project and scrolls to the milestone", async () => {
  const p = await addProject({ name: "Book", content: "## Draft\n- [ ] a\n## Review\n- [ ] b\n" });
  await markNext(p.id, "b");
  const scroll = vi.spyOn(Element.prototype, "scrollIntoView");
  const { user } = renderApp();

  await screen.findByRole("group", { name: "Next up" });
  await user.click(await nextUp().findByRole("button", { name: /^Book/ }));

  await screen.findByRole("button", { name: "Close project" });
  await waitFor(() => expect(scroll).toHaveBeenCalled());
  const contexts = scroll.mock.contexts;
  expect(contexts[contexts.length - 1]).toHaveTextContent(/^Review/);
  scroll.mockRestore();
});

test("Next up follows marking and checking tasks", async () => {
  await addProject({ name: "P", content: "## M\n- [ ] alpha\n- [ ] beta\n" });
  const { user } = renderApp();
  expect(await screen.findByText("Nothing marked next")).toBeInTheDocument();
  await openProject(user, "P");

  await user.click(nextButton("alpha"));
  await waitFor(() => expect(rows()).toEqual(["P1 nextM0 of 2 done"]));

  await user.click(checkbox("alpha"));
  await waitFor(() => expect(screen.getByText("Nothing marked next")).toBeInTheDocument());
});
