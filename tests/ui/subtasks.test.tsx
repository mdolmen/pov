import { readFileSync } from "node:fs";
import { waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, checkbox, doneSection, openProject, renderApp } from "./helpers";

const TODO = "- [ ] parent\n  - [ ] one\n  - [ ] two\n- [ ] other\n";

test("checking a subtask writes only that subtask", async () => {
  const project = await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(checkbox("one"));

  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe(
      "- [ ] parent\n  - [x] one\n  - [ ] two\n- [ ] other\n",
    ),
  );
  expect(doneSection()).toBeNull();
});

test("checking the last open subtask completes the parent", async () => {
  const project = await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(checkbox("one"));
  await waitFor(() => expect(readFileSync(project.file_path, "utf8")).toContain("[x] one"));
  await user.click(checkbox("two"));

  await waitFor(() => expect(doneSection()?.getByText("parent")).toBeInTheDocument());
  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe(
      "- [x] parent\n  - [x] one\n  - [x] two\n- [ ] other\n",
    ),
  );
});

test("checking or unchecking a parent does the same to all its subtasks", async () => {
  const project = await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(checkbox("parent"));
  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe(
      "- [x] parent\n  - [x] one\n  - [x] two\n- [ ] other\n",
    ),
  );
  await waitFor(() => expect(doneSection()?.getByText("two")).toBeInTheDocument());

  await user.click(checkbox("parent"));
  await waitFor(() => expect(readFileSync(project.file_path, "utf8")).toBe(TODO));
  await waitFor(() => expect(doneSection()).toBeNull());
});
