import { writeFileSync } from "node:fs";
import { screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import {
  addProject,
  checkbox,
  doneSection,
  inOrder,
  nextButton,
  openProject,
  renderApp,
  selectTask,
  taskHash,
} from "./helpers";

const TODO = "- [ ] alpha\n- [ ] beta\n- [ ] gamma\n";

test("+ moves a task to the top, × moves it back", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(nextButton("gamma"));
  expect(inOrder("gamma", "alpha", "beta")).toBe(true);

  await user.click(nextButton("gamma"));
  expect(inOrder("alpha", "beta", "gamma")).toBe(true);
});

test("double-clicking a task selects it, double-clicking again unselects it", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.dblClick(screen.getByText("beta"));
  expect(inOrder("beta", "alpha", "gamma")).toBe(true);

  await user.dblClick(screen.getByText("beta"));
  expect(inOrder("alpha", "beta", "gamma")).toBe(true);
});

test("next tasks are saved and counted on the project card", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  await user.click(nextButton("beta"));
  await user.click(nextButton("gamma"));

  await user.click(screen.getByRole("button", { name: "Back" }));
  expect(await screen.findByRole("button", { name: /^P2/ })).toBeInTheDocument();

  await openProject(user, "P");
  await screen.findByText("alpha");
  expect(inOrder("beta", "gamma", "alpha")).toBe(true);
});

test("checking a next task sends it to Done and unselects it", async () => {
  await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");
  await user.click(nextButton("beta"));

  await user.click(checkbox("beta"));

  await waitFor(() => expect(doneSection()?.getByText("beta")).toBeInTheDocument());
  await user.click(screen.getByRole("button", { name: "Back" }));
  const card = await screen.findByRole("button", { name: /^P/ });
  await waitFor(() => expect(card).toHaveTextContent(/^P2 tasks/));
});

test("refresh clears next tasks checked outside the app", async () => {
  const project = await addProject({ name: "P", content: TODO });
  await selectTask(project.id, await taskHash(project.id, "alpha"));
  writeFileSync(project.file_path, "- [x] alpha\n- [ ] beta\n- [ ] gamma\n");
  const { user } = renderApp();
  expect(await screen.findByRole("button", { name: /^P1/ })).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: "Refresh activity" }));

  await waitFor(
    () => expect(screen.getByRole("button", { name: /^P/ })).toHaveTextContent(/^P2 tasks/),
    { timeout: 5000 },
  );
});
