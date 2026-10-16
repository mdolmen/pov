import { readFileSync } from "node:fs";
import { waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { addProject, checkbox, openProject, renderApp, struck } from "./helpers";

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
  expect(struck("parent")).toBe(false);
});

test("checking the last open subtask leaves the parent open", async () => {
  const project = await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(checkbox("one"));
  await waitFor(() => expect(readFileSync(project.file_path, "utf8")).toContain("[x] one"));
  await user.click(checkbox("two"));

  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe(
      "- [ ] parent\n  - [x] one\n  - [x] two\n- [ ] other\n",
    ),
  );
  expect(struck("two")).toBe(true);
  expect(struck("parent")).toBe(false);
});

test("checking a parent checks its subtasks, unchecking leaves them", async () => {
  const project = await addProject({ name: "P", content: TODO });
  const { user } = renderApp();
  await openProject(user, "P");

  await user.click(checkbox("parent"));
  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe(
      "- [x] parent\n  - [x] one\n  - [x] two\n- [ ] other\n",
    ),
  );
  await waitFor(() => expect(struck("parent")).toBe(true));
  expect(struck("two")).toBe(true);

  await user.click(checkbox("parent"));
  await waitFor(() =>
    expect(readFileSync(project.file_path, "utf8")).toBe(
      "- [ ] parent\n  - [x] one\n  - [x] two\n- [ ] other\n",
    ),
  );
  await waitFor(() => expect(struck("parent")).toBe(false));
  expect(struck("two")).toBe(true);
});
