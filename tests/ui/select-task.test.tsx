import { writeFileSync } from "node:fs";
import { screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import {
  addProject,
  checkbox,
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
