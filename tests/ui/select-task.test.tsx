import { writeFileSync } from "node:fs";
import { screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import {
  addProject,
  checkbox,
  inboxTasks,
  inOrder,
  nextButton,
  openProject,
  projectTiles,
  renderApp,
  selectTask,
  struck,
  taskHash,
  taskPane,
  tileName,
} from "./helpers";

/** Whether a task carries the violet "next" band. */
const banded = (text: string) =>
  (taskPane().getByText(text).closest("[data-task]") as HTMLElement).style.boxShadow !== "";

const TODO = "- [ ] alpha\n- [ ] beta\n- [ ] gamma\n";

test("Mark next bands a task in place, Next unmarks it", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  expect(nextButton("gamma")).toHaveTextContent("Mark next");

  await user.click(nextButton("gamma"));
  expect(banded("gamma")).toBe(true);
  expect(nextButton("gamma")).toHaveTextContent("Next");
  expect(inOrder("alpha", "beta", "gamma")).toBe(true);

  await user.click(nextButton("gamma"));
  expect(banded("gamma")).toBe(false);
});

test("double-clicking a task selects it, double-clicking again unselects it", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.dblClick(taskPane().getByText("beta"));
  expect(banded("beta")).toBe(true);

  await user.dblClick(taskPane().getByText("beta"));
  expect(banded("beta")).toBe(false);
});

test("next tasks are saved and counted on the project card", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  await user.click(nextButton("beta"));
  await user.click(nextButton("gamma"));

  expect(await projectTiles().findByRole("button", { name: /^P2/ })).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Close project" }));
  await openProject(user, "P");
  expect(banded("beta") && banded("gamma") && !banded("alpha")).toBe(true);
});

test("checking a next task strikes it and unselects it", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  await user.click(nextButton("beta"));

  await user.click(checkbox("beta"));

  await waitFor(() => expect(struck("beta")).toBe(true));
  expect(banded("beta")).toBe(false);
  await waitFor(() =>
    expect(projectTiles().getByRole("button", { name: tileName("P") })).toHaveTextContent(/^P2 open/),
  );
});

test("a next task checked outside the app no longer counts, before and after a refresh", async () => {
  const project = await addProject({ name: "P", content: TODO });
  await selectTask(project.id, await taskHash(project.id, "alpha"));
  writeFileSync(project.file_path, "- [x] alpha\n- [ ] beta\n- [ ] gamma\n");
  const { user } = renderApp();
  expect(await projectTiles().findByRole("button", { name: /^P2 open/ })).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Refresh activity" }));

  await waitFor(
    () => expect(projectTiles().getByRole("button", { name: tileName("P") })).toHaveTextContent(/^P2 open/),
    { timeout: 5000 },
  );
});

test("Next shows only the next tasks, under their milestones", async () => {
  const project = await addProject({
    name: "P",
    content: "## One\n- [ ] alpha\n- [x] beta\n## Two\n- [ ] gamma\n- [ ] delta\n",
  });
  await selectTask(project.id, await taskHash(project.id, "gamma"));
  await inboxTasks(project.id, "delta");
  const { user } = renderApp();
  await openProject(user, "P");

  const filter = screen.getByRole("button", { name: "Next", pressed: false });
  await user.click(filter);

  expect(filter).toHaveAttribute("aria-pressed", "true");
  expect(taskPane().getByText("gamma")).toBeInTheDocument();
  for (const text of ["alpha", "beta", "delta"]) {
    expect(taskPane().queryByText(text)).not.toBeInTheDocument();
  }
  expect(taskPane().getByText("Two")).toBeInTheDocument();
  expect(taskPane().queryByText("One")).not.toBeInTheDocument();

  // Inbox replaces it: one filter at a time.
  await user.click(screen.getByRole("button", { name: "Inbox", pressed: false }));
  expect(filter).toHaveAttribute("aria-pressed", "false");
  expect(taskPane().getByText("delta")).toBeInTheDocument();
  expect(taskPane().queryByText("gamma")).not.toBeInTheDocument();
});

test("no Next filter without next tasks; finishing the last one turns it off", async () => {
  await addProject({ name: "P", content: "- [ ] alpha\n- [ ] beta\n" });
  const { user } = renderApp();
  await openProject(user, "P");
  expect(screen.queryByRole("button", { name: "Next", pressed: false })).not.toBeInTheDocument();

  await user.click(nextButton("alpha"));
  await user.click(screen.getByRole("button", { name: "Next", pressed: false }));
  await user.click(checkbox("alpha"));

  await waitFor(() =>
    expect(screen.queryByRole("button", { name: "Next", pressed: true })).not.toBeInTheDocument(),
  );
  expect(taskPane().getByText("beta")).toBeInTheDocument();
});
